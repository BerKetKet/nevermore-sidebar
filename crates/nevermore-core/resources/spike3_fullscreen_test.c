#include <gtk/gtk.h>
#include <stdio.h>

static void on_activate(GtkApplication *app, gpointer user_data) {
    GtkWidget *win = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(win), "Nevermore Spike 3 Fullscreen Test Target");

    GtkCssProvider *css = gtk_css_provider_new();
    gtk_css_provider_load_from_string(css,
        "window { background-color: #064e3b; color: #34d399; }"
        "label { font-size: 24px; font-weight: bold; font-family: Inter, sans-serif; }"
    );
    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        GTK_STYLE_PROVIDER(css),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    GtkWidget *label = gtk_label_new("FULLSCREEN TEST APPLICATION (Simulated Game)\n\nVerifying TOP Layer Sidebar is Occluded Beneath Fullscreen Window");
    gtk_window_set_child(GTK_WINDOW(win), label);

    // Enter fullscreen mode
    gtk_window_fullscreen(GTK_WINDOW(win));
    gtk_window_present(GTK_WINDOW(win));

    printf("[SPIKE 3 FULLSCREEN READY] Fullscreen window mapped and active\n");
    fflush(stdout);
}

int main(int argc, char **argv) {
    GtkApplication *app = gtk_application_new("io.github.nevermore.fullscreen_test", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(on_activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
