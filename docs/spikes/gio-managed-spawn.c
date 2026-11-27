/*
 * gio-managed-spawn: launch a desktop file through GIO's launch manager.
 *
 * This tool is meant to be started by docs/spikes/session-supervisor. The
 * supervisor is the subreaper; this tool only performs the GIO launch and
 * exits, so orphaned descendants are adopted by the supervisor.
 *
 * Build:
 *   gcc -O2 -Wall -Wextra -o /tmp/gio-managed-spawn docs/spikes/gio-managed-spawn.c \
 *     $(pkg-config --cflags --libs gio-unix-2.0)
 *
 * Usage:
 *   gio-managed-spawn [--do-not-reap] [--stdout-null] /absolute/file.desktop
 *
 * Output is one JSON object per line on stdout.
 */
#include <gio/gdesktopappinfo.h>
#include <glib.h>
#include <stdio.h>
#include <string.h>

static gint64 t0;
static int callback_seen = 0;
static int callback_pid = -1;

static void
launch_cb(GDesktopAppInfo *appinfo, GPid pid, gpointer user_data)
{
    (void)appinfo;
    (void)user_data;
    gint64 us = g_get_monotonic_time();
    callback_seen = 1;
    callback_pid = (int)pid;
    g_print("{\"event\":\"gio_callback\",\"pid\":%d,\"dt_ms\":%" G_GINT64_FORMAT "}\n",
            (int)pid, (us - t0) / 1000);
    fflush(stdout);
}

int
main(int argc, char **argv)
{
    gboolean do_not_reap = FALSE;
    gboolean stdout_null = FALSE;
    const char *path = NULL;

    for (int i = 1; i < argc; ++i) {
        if (strcmp(argv[i], "--do-not-reap") == 0) {
            do_not_reap = TRUE;
        } else if (strcmp(argv[i], "--stdout-null") == 0) {
            stdout_null = TRUE;
        } else if (argv[i][0] == '-') {
            g_printerr("unknown option: %s\n", argv[i]);
            return 2;
        } else {
            path = argv[i];
        }
    }
    if (path == NULL) {
        g_printerr("usage: %s [--do-not-reap] [--stdout-null] /absolute/file.desktop\n",
                   argv[0]);
        return 2;
    }

    GDesktopAppInfo *info = g_desktop_app_info_new_from_filename(path);
    if (info == NULL) {
        g_print("{\"event\":\"gio_load_failed\",\"path\":\"%s\"}\n", path);
        return 3;
    }

    GSpawnFlags flags = G_SPAWN_SEARCH_PATH;
    if (do_not_reap)
        flags |= G_SPAWN_DO_NOT_REAP_CHILD;
    if (stdout_null)
        flags |= G_SPAWN_STDOUT_TO_DEV_NULL | G_SPAWN_STDERR_TO_DEV_NULL;

    GAppLaunchContext *ctx = g_app_launch_context_new();
    GError *err = NULL;
    t0 = g_get_monotonic_time();
    gboolean ok = g_desktop_app_info_launch_uris_as_manager(
        info, NULL, ctx, flags, NULL, NULL, launch_cb, NULL, &err);
    gint64 us = g_get_monotonic_time();

    g_print("{\"event\":\"gio_result\",\"ok\":%s,\"callback\":%s,\"callback_pid\":%d,"
            "\"total_ms\":%" G_GINT64_FORMAT ",\"error\":\"%s\"}\n",
            ok ? "true" : "false",
            callback_seen ? "true" : "false",
            callback_pid,
            (us - t0) / 1000,
            err ? err->message : "");
    fflush(stdout);

    if (err != NULL)
        g_error_free(err);
    g_object_unref(ctx);
    g_object_unref(info);
    return ok ? 0 : 1;
}
