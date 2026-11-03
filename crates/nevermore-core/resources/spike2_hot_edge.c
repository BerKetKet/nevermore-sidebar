#include <gtk/gtk.h>
#include <gtk4-layer-shell.h>
#include <stdio.h>
#include <stdlib.h>

static int enter_count = 0;

static void on_enter(GtkEventControllerMotion *controller, double x, double y, gpointer user_data) {
    enter_count++;
    printf("[HOT EDGE ENTER] PointerEnteredEdge at x=%.1f, y=%.1f (Total triggers: %d)\n", x, y, enter_count);
    fflush(stdout);
}

static void on_activate(GtkApplication *app, gpointer user_data) {
    GtkWidget *win = gtk_application_window_new(app);
    GtkWindow *layer_window = GTK_WINDOW(win);

    gtk_layer_init_for_window(layer_window);
    gtk_layer_set_layer(layer_window, GTK_LAYER_SHELL_LAYER_TOP);
    gtk_layer_set_namespace(layer_window, "nevermore-hotedge");

    // Anchor to right edge, top to bottom
    gtk_layer_set_anchor(layer_window, GTK_LAYER_SHELL_EDGE_RIGHT, TRUE);
    gtk_layer_set_anchor(layer_window, GTK_LAYER_SHELL_EDGE_TOP, TRUE);
    gtk_layer_set_anchor(layer_window, GTK_LAYER_SHELL_EDGE_BOTTOM, TRUE);
    gtk_layer_set_anchor(layer_window, GTK_LAYER_SHELL_EDGE_LEFT, FALSE);

    // 2-pixel trigger strip with zero strut
    gtk_layer_set_exclusive_zone(layer_window, 0);
    gtk_layer_set_keyboard_mode(layer_window, GTK_LAYER_SHELL_KEYBOARD_MODE_NONE);
    gtk_window_set_default_size(layer_window, 2, 1080);

    // Completely transparent input-only strip
    GtkCssProvider *css = gtk_css_provider_new();
    gtk_css_provider_load_from_string(css,
        "window { background-color: rgba(0, 0, 0, 0.01); }"
    );
    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        GTK_STYLE_PROVIDER(css),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    GtkEventController *motion = gtk_event_controller_motion_new();
    g_signal_connect(motion, "enter", G_CALLBACK(on_enter), NULL);
    gtk_widget_add_controller(win, motion);

    gtk_window_present(layer_window);
    printf("[HOT EDGE READY] 2px right-edge trigger strip mapped on KWin 6.8.2\n");
    fflush(stdout);
}

int main(int argc, char **argv) {
    GtkApplication *app = gtk_application_new("io.github.nevermore.hotedge", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(on_activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
