#!/usr/bin/env python3
"""Small native GTK4 app for CapyDeck/CouchDeck Direct/Managed session spikes.

Modes:
  stay      - show one window and keep running
  recreate  - destroy window #1 after 3 s and create window #2 after 2 s

The app spawns a `sleep 300` child to exercise cgroup/child cleanup.
Run it under a systemd transient service with KillMode=control-group.
"""
import os
import subprocess
import sys

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk

mode = sys.argv[1] if len(sys.argv) > 1 else "stay"
child_seconds = sys.argv[2] if len(sys.argv) > 2 else "300"
child = subprocess.Popen(["sleep", child_seconds])
print(f"TESTAPP pid={os.getpid()} child={child.pid} mode={mode}", flush=True)

app = Gtk.Application(application_id="de.capydeck.TestApp")
app.hold()


def on_close(window):
    app.release()
    app.quit()
    return False


def make_window(number):
    window = Gtk.ApplicationWindow(application=app)
    window.set_title(f"CapyDeck TestApp {mode} #{number}")
    window.set_default_size(640, 360)
    label = Gtk.Label(
        label=(
            f"CapyDeck TestApp\nmode={mode}\nwindow #{number}\n"
            f"pid={os.getpid()}\nchild={child.pid}"
        )
    )
    window.set_child(label)
    window.connect("close-request", on_close)
    window.present()
    return window


def recreate(window):
    window.destroy()
    GLib.timeout_add(2000, lambda: (make_window(2), False)[1])
    return False


window = make_window(1)
if mode == "recreate":
    GLib.timeout_add(3000, recreate, window)

app.run(None)
