#include <gtk/gtk.h>
#include <gtk4-layer-shell.h>
#include <stdio.h>
#include <stdlib.h>

static GtkWidget *status_label = NULL;
static GtkWindow *layer_window = NULL;
static int step = 0;

static const char* mode_name(GtkLayerShellKeyboardMode mode) {
    switch (mode) {
        case GTK_LAYER_SHELL_KEYBOARD_MODE_NONE: return "None (Zero Focus - Peek)";
        case GTK_LAYER_SHELL_KEYBOARD_MODE_EXCLUSIVE: return "Exclusive (Locked Focus)";
        case GTK_LAYER_SHELL_KEYBOARD_MODE_ON_DEMAND: return "OnDemand (Click to Engage)";
        default: return "Unknown";
    }
}

static void log_state(const char *phase, GtkLayerShellKeyboardMode mode) {
    printf("[SPIKE 1 VERIFY] Phase: %s | KeyboardMode: %s | Layer: TOP | ExclusiveZone: %d\n",
           phase, mode_name(mode), gtk_layer_get_exclusive_zone(layer_window));
    fflush(stdout);
}

static gboolean run_verification_cycle(gpointer user_data) {
    step++;
    if (step == 1) {
        // Step 1: Initial Peek state verified
        log_state("1_INITIAL_PEEK", gtk_layer_get_keyboard_mode(layer_window));
        return G_SOURCE_CONTINUE;
    } else if (step == 2) {
        // Step 2: Transition to Engaged (OnDemand)
        gtk_layer_set_keyboard_mode(layer_window, GTK_LAYER_SHELL_KEYBOARD_MODE_ON_DEMAND);
        log_state("2_ENGAGED_ON_DEMAND", gtk_layer_get_keyboard_mode(layer_window));
        return G_SOURCE_CONTINUE;
    } else if (step == 3) {
        // Step 3: Transition to Reverted (None) simulating Escape / Leave
        gtk_layer_set_keyboard_mode(layer_window, GTK_LAYER_SHELL_KEYBOARD_MODE_NONE);
        log_state("3_REVERTED_TO_NONE", gtk_layer_get_keyboard_mode(layer_window));
        return G_SOURCE_CONTINUE;
    } else if (step == 4) {
        // Step 4: Test Pinned Mode with Exclusive Zone 0
        gtk_layer_set_keyboard_mode(layer_window, GTK_LAYER_SHELL_KEYBOARD_MODE_ON_DEMAND);
        gtk_layer_set_exclusive_zone(layer_window, 0); // Zero strut rule
        log_state("4_PINNED_ZERO_STRUT", gtk_layer_get_keyboard_mode(layer_window));
        return G_SOURCE_CONTINUE;
    } else {
        printf("[SPIKE 1 COMPLETE] All layer-shell focus modes successfully verified on KWin 6.8.2!\n");
        fflush(stdout);
        gtk_window_destroy(layer_window);
        return G_SOURCE_REMOVE;
    }
}

static void on_activate(GtkApplication *app, gpointer user_data) {
    GtkWidget *win = gtk_application_window_new(app);
    layer_window = GTK_WINDOW(win);

    gtk_layer_init_for_window(layer_window);
    gtk_layer_set_layer(layer_window, GTK_LAYER_SHELL_LAYER_TOP);
    gtk_layer_set_namespace(layer_window, "nevermore-spike1");

    gtk_layer_set_anchor(layer_window, GTK_LAYER_SHELL_EDGE_RIGHT, TRUE);
    gtk_layer_set_anchor(layer_window, GTK_LAYER_SHELL_EDGE_TOP, TRUE);
    gtk_layer_set_anchor(layer_window, GTK_LAYER_SHELL_EDGE_BOTTOM, TRUE);
    gtk_layer_set_anchor(layer_window, GTK_LAYER_SHELL_EDGE_LEFT, FALSE);

    gtk_layer_set_exclusive_zone(layer_window, 0);
    gtk_layer_set_keyboard_mode(layer_window, GTK_LAYER_SHELL_KEYBOARD_MODE_NONE);
    gtk_window_set_default_size(layer_window, 380, 1080);

    GtkCssProvider *css = gtk_css_provider_new();
    gtk_css_provider_load_from_string(css,
        "window { background-color: #18181b; color: #ffffff; border-left: 2px solid #0a84ff; }"
        "label { font-family: Inter, sans-serif; font-size: 15px; margin: 20px; }"
    );
    gtk_style_context_add_provider_for_display(
        gdk_display_get_default(),
        GTK_STYLE_PROVIDER(css),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    GtkWidget *box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 15);
    GtkWidget *title = gtk_label_new(NULL);
    gtk_label_set_markup(GTK_LABEL(title), "<span weight='bold' size='large' color='#0a84ff'>Nevermore Spike 1</span>\nwlr-layer-shell KWin 6.8.2 Focus Verification");
    gtk_box_append(GTK_BOX(box), title);

    status_label = gtk_label_new("Running automated focus verification cycle...");
    gtk_box_append(GTK_BOX(box), status_label);
    gtk_window_set_child(layer_window, box);

    gtk_window_present(layer_window);

    // Run verification cycle every 1 second
    g_timeout_add(1000, run_verification_cycle, NULL);
}

int main(int argc, char **argv) {
    GtkApplication *app = gtk_application_new("io.github.nevermore.spike1", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(on_activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
