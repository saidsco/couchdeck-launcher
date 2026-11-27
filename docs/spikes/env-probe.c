/*
 * env-probe: print selected launch-environment details for supervisor research.
 * It deliberately does NOT dump the full environment (no secrets).
 *
 * Build:
 *   gcc -O2 -Wall -Wextra -o /tmp/env-probe docs/spikes/env-probe.c
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/prctl.h>
#include <sys/stat.h>

#ifndef PR_GET_CHILD_SUBREAPER
#define PR_GET_CHILD_SUBREAPER 37
#endif

static void print_fd(const char *name, int fd)
{
    char link[64];
    char target[512];
    snprintf(link, sizeof(link), "/proc/self/fd/%d", fd);
    ssize_t n = readlink(link, target, sizeof(target) - 1);
    if (n >= 0) {
        target[n] = '\0';
        printf("fd_%s=%s\n", name, target);
    } else {
        printf("fd_%s=closed\n", name);
    }
    printf("tty_%s=%d\n", name, isatty(fd));
}

static void print_env(const char *key)
{
    const char *value = getenv(key);
    printf("env_%s=%s\n", key, value ? value : "<unset>");
}

int main(int argc, char **argv)
{
    char cwd[1024];
    char cgroup[1024];
    int subreaper = -1;
    FILE *f;

    printf("pid=%d\n", (int)getpid());
    printf("ppid=%d\n", (int)getppid());
    printf("pgrp=%d\n", (int)getpgrp());
    printf("sid=%d\n", (int)getsid(0));
    printf("cwd=%s\n", getcwd(cwd, sizeof(cwd)) ? cwd : "<error>");
    printf("argv0=%s\n", argv[0]);
    printf("argc=%d\n", argc);

    if (prctl(PR_GET_CHILD_SUBREAPER, &subreaper, 0, 0, 0) == 0)
        printf("child_subreaper=%d\n", subreaper);
    else
        printf("child_subreaper=<error>\n");

    f = fopen("/proc/self/cgroup", "r");
    if (f) {
        if (fgets(cgroup, sizeof(cgroup), f))
            printf("cgroup=%s", cgroup);
        fclose(f);
    }

    print_fd("stdin", 0);
    print_fd("stdout", 1);
    print_fd("stderr", 2);

    print_env("PATH");
    print_env("HOME");
    print_env("XDG_RUNTIME_DIR");
    print_env("XDG_SESSION_TYPE");
    print_env("XDG_CURRENT_DESKTOP");
    print_env("WAYLAND_DISPLAY");
    print_env("DISPLAY");
    print_env("DBUS_SESSION_BUS_ADDRESS");
    print_env("PULSE_SERVER");
    print_env("PIPEWIRE_REMOTE");
    print_env("SDL_VIDEODRIVER");
    print_env("LD_LIBRARY_PATH");
    print_env("LD_PRELOAD");

    printf("env_STEAM_COMPAT_APP_ID=%s\n", getenv("STEAM_COMPAT_APP_ID") ? "present" : "<unset>");
    printf("env_PROTON_VERSION=%s\n", getenv("PROTON_VERSION") ? "present" : "<unset>");
    return 0;
}
