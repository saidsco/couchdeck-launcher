/*
 * GIO launch-manager PID-callback spike for CapyDeck/CouchDeck research.
 *
 * Build:
 *   gcc -O2 -Wall -Wextra -o /tmp/gio-pid-spike docs/spikes/gio-pid-spike.c \
 *     $(pkg-config --cflags --libs gio-unix-2.0)
 *
 * Usage:
 *   /tmp/gio-pid-spike /absolute/path/to/app.desktop
 *
 * The program prints:
 *   LOADED ...        - desktop file parsed successfully
 *   CALLBACK pid=...  - PID passed to GDesktopAppLaunchCallback
 *   RESULT ok=...     - result of launch_uris_as_manager
 *
 * Research result: the callback is the immediate spawned process, not
 * necessarily the game/application process. See GENERIC-GIO-SPIKE-RESULTS.md.
 */
#include <gio/gdesktopappinfo.h>
#include <glib.h>
#include <stdio.h>

static gint64 t0;

static void
launch_cb(GDesktopAppInfo *appinfo, GPid pid, gpointer user_data)
{
    (void)appinfo;
    (void)user_data;
    gint64 us = g_get_monotonic_time();
    g_print("CALLBACK pid=%d dt_ms=%" G_GINT64_FORMAT "\n",
            (int)pid, (us - t0) / 1000);
}

int
main(int argc, char **argv)
{
    if (argc < 2) {
        g_printerr("usage: %s /absolute/file.desktop\n", argv[0]);
        return 2;
    }

    GDesktopAppInfo *info = g_desktop_app_info_new_from_filename(argv[1]);
    if (info == NULL) {
        g_printerr("LOAD_FAILED %s\n", argv[1]);
        return 3;
    }

    gchar *tryexec = g_desktop_app_info_get_string(info, "TryExec");
    g_print("LOADED id=%s name=%s hidden=%d dbusactivatable=%d tryexec=%s\n",
            g_app_info_get_id(G_APP_INFO(info)),
            g_app_info_get_name(G_APP_INFO(info)),
            g_desktop_app_info_get_is_hidden(info),
            g_desktop_app_info_get_boolean(info, "DBusActivatable"),
            tryexec ? tryexec : "-");
    g_free(tryexec);

    GAppLaunchContext *ctx = g_app_launch_context_new();
    GError *err = NULL;
    t0 = g_get_monotonic_time();

    /*
     * Redirect child stdout/stderr so the child does not keep a shell pipe
     * open. This is important for automation and for tools that capture output.
     */
    gboolean ok = g_desktop_app_info_launch_uris_as_manager(
        info,
        NULL,
        ctx,
        G_SPAWN_SEARCH_PATH | G_SPAWN_DO_NOT_REAP_CHILD |
            G_SPAWN_STDOUT_TO_DEV_NULL | G_SPAWN_STDERR_TO_DEV_NULL,
        NULL,
        NULL,
        launch_cb,
        NULL,
        &err);

    gint64 us = g_get_monotonic_time();
    g_print("RESULT ok=%d total_ms=%" G_GINT64_FORMAT " err=%s\n",
            ok, (us - t0) / 1000, err ? err->message : "-");

    if (err != NULL)
        g_error_free(err);
    g_object_unref(ctx);
    g_object_unref(info);
    return ok ? 0 : 1;
}
