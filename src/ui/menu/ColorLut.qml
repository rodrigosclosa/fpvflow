// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

import QtQuick
// Namespaced because a bare FolderDialog is ambiguous - Qt ships two.
import QtQuick.Dialogs as QQD

import "../components/"

/**
 * Color panel.
 *
 * Loads a conversion LUT in the Adobe Cube format - the log to Rec.709 tables
 * cameras ship - and applies it during processing, so the exported file is
 * already graded and does not need a second pass in an editor.
 *
 * The LUT runs after the warp and before the drawing overlays, so the safe area
 * and any drawing keep their own colors.
 */
MenuItem {
    id: root;
    text: qsTr("Color");
    iconName: "stars5";
    objectName: "colorlut";
    innerItem.enabled: window.videoArea.vid.loaded;

    /// Whether a LUT is loaded. Controls the visibility of everything below the button.
    readonly property bool hasLut: !!lutInfo.text;

    /// Describes the loaded table. The core returns JSON so the wording can be
    /// translated here instead of shipping English to every language.
    function lutSummary(json: string): string {
        if (!json) return "";
        const info = JSON.parse(json);
        // A title is optional in the .cube format, so there are two sentences
        // rather than one with an empty slot.
        return info.title
            ? qsTr("%1 - %2³ entries").arg(info.title).arg(info.size)
            : qsTr("%1³ entries").arg(info.size);
    }

    /// What goes into the `.gyroflow` file under `color_lut`.
    ///
    /// The internal URL, not the readable path: it is what reopens the file, and
    /// on Android the two are not interchangeable.
    /// The sliders, by the name the core knows them under. One list drives
    /// saving, loading and resetting, so the three cannot drift apart.
    readonly property var adjustmentSliders: ({
        "exposure":    adjExposure,
        "contrast":    adjContrast,
        "saturation":  adjSaturation,
        "temperature": adjTemperature,
        "tint":        adjTint,
        "highlights":  adjHighlights,
        "shadows":     adjShadows,
        "vignette":    adjVignette
    });

    /// Puts every slider back to neutral.
    function resetAdjustments(): void {
        for (const name in root.adjustmentSliders) root.adjustmentSliders[name].value = 0;
        controller.reset_color_adjustments();
    }

    function getSettings(): var {
        const adjustments = { };
        let any = false;
        for (const name in root.adjustmentSliders) {
            const v = root.adjustmentSliders[name].value;
            // Only what differs from neutral, so a project with no grading does
            // not carry eight zeroes.
            if (v !== 0) { adjustments[name] = v; any = true; }
        }

        const url = controller.get_color_lut_url();
        const out = { };
        if (url) { out.url = url; out.amount = controller.get_color_lut_amount(); }
        if (any) out.adjustments = adjustments;
        return out;
    }

    /// Restores from a project. Missing or empty keys leave the panel untouched,
    /// so older projects load unchanged.
    function loadGyroflow(obj: var): void {
        const o = obj.color_lut;
        if (!o) return;

        if (o.url) {
            controller.load_color_lut_url(o.url);
            // Before, 100% was the only option, so a project without the key
            // means full strength rather than none.
            lutAmount.value = o.hasOwnProperty("amount")? +o.amount : 1.0;
        }

        // Absent means neutral, and the sliders are reset first so a project
        // without grading clears whatever the previous clip left behind.
        root.resetAdjustments();
        if (o.adjustments) {
            for (const name in root.adjustmentSliders) {
                if (o.adjustments.hasOwnProperty(name)) {
                    root.adjustmentSliders[name].value = +o.adjustments[name];
                }
            }
        }
    }

    /// Folder scanned for the library dropdown. Global, not per project.
    property string lutFolder: settings.value("lutFolder", "");

    /// Rescans `lutFolder` and refreshes the dropdown.
    ///
    /// Called on load and after the folder changes, not on every repaint: it
    /// touches the filesystem, and a folder of LUTs does not change while the
    /// panel is open.
    function refreshLibrary(): void {
        if (!root.lutFolder) { libraryModel = []; return; }
        const json = controller.list_color_luts(root.lutFolder);
        libraryModel = json? JSON.parse(json) : [];
    }
    property var libraryModel: [];

    Component.onCompleted: root.refreshLibrary();

    FileDialog {
        id: lutFileDialog;
        title: qsTr("Choose a LUT file");
        nameFilters: [qsTr("LUT files") + " (*.cube *.CUBE)"];
        type: "lut";
        onAccepted: controller.load_color_lut(lutFileDialog.selectedFile);
    }

    // ---- Library ----
    // A dropdown over a folder the user picks once, so the LUTs they actually
    // use are two clicks away instead of a file dialog every time.
    Label {
        text: qsTr("Library");
        visible: root.libraryModel.length > 0;
        ComboBox {
            id: library;
            // Index 0 is the placeholder, so the list never starts on a LUT the
            // user did not choose.
            model: [qsTr("Choose a LUT...")].concat(root.libraryModel.map(x => x.name));
            width: parent.width;
            currentIndex: 0;
            onCurrentIndexChanged: {
                if (currentIndex > 0) {
                    const item = root.libraryModel[currentIndex - 1];
                    if (item) controller.load_color_lut_url(item.url);
                }
            }
        }
    }

    QQD.FolderDialog {
        id: lutFolderDialog;
        title: qsTr("Choose the LUT folder");
        onAccepted: {
            root.lutFolder = selectedFolder.toString();
            settings.setValue("lutFolder", root.lutFolder);
            root.refreshLibrary();
        }
    }

    Button {
        text: root.lutFolder? qsTr("Change LUT folder") : qsTr("Set LUT folder");
        iconName: "folder";
        width: parent.width;
        tooltip: qsTr("The .cube files in this folder are listed above. The folder is remembered between sessions.");
        onClicked: lutFolderDialog.open();
    }

    Button {
        text: root.hasLut? qsTr("Replace LUT") : qsTr("Load LUT");
        iconName: "stars5";
        width: parent.width;
        onClicked: lutFileDialog.open2();
    }

    BasicText {
        id: lutInfo;
        width: parent.width;
        wrapMode: Text.WordWrap;
        leftPadding: 0;
        text: root.lutSummary(controller.get_color_lut_info());
        visible: !!text;

        Connections {
            target: controller;

            // Covers loading, clearing and failing: the panel is rebuilt from
            // the core's state rather than from what the click intended.
            function onColor_lut_changed(ok: bool, error: string): void {
                lutInfo.text = root.lutSummary(controller.get_color_lut_info());
                lutPath.text = controller.get_color_lut_path();
                lutError.text = ok? "" : error;
            }
        }
    }

    BasicText {
        id: lutPath;
        width: parent.width;
        wrapMode: Text.WrapAnywhere;
        leftPadding: 0;
        font.pixelSize: 11 * dpiScale;
        opacity: 0.6;
        text: controller.get_color_lut_path();
        visible: !!text;
    }

    // A LUT the user picked and that could not be read is worth showing in the
    // panel: the message names the line, which is what makes a truncated file
    // recognizable as such.
    BasicText {
        id: lutError;
        width: parent.width;
        wrapMode: Text.WordWrap;
        leftPadding: 0;
        color: styleTextColorError;
        visible: !!text;
    }

    // Dragging this only rewrites a uniform - no re-upload, no pipeline rebuild -
    // so it stays responsive while the preview redraws.
    Label {
        text: qsTr("Intensity");
        visible: root.hasLut;
        SliderWithField {
            id: lutAmount;
            // `value` is the unscaled 0..1 number the controller wants; `scaler`
            // only affects the number shown in the field, which reads 0..100 %.
            // Same split as marginPixels in Advanced.qml.
            value: 1.0;
            defaultValue: 100;
            from: 0;
            to: 1;
            unit: "%";
            precision: 0;
            width: parent.width;
            scaler: 100.0;
            onValueChanged: controller.set_color_lut_amount(value);
        }
    }

    Button {
        text: qsTr("Remove LUT");
        iconName: "bin";
        width: parent.width;
        visible: root.hasLut;
        onClicked: {
            controller.clear_color_lut();
            // Back to the placeholder, otherwise the dropdown keeps naming a LUT
            // that is no longer applied.
            library.currentIndex = 0;
        }
    }

    // ---- Adjustments ----
    // These run after the LUT, on Rec.709 values, so they behave the way a
    // grading panel does rather than fighting the log curve. Collapsed by
    // default: the common case is a LUT and nothing else.
    AdvancedSection {
        btn.text: qsTr("Adjustments");

        Label {
            text: qsTr("Exposure");
            SliderWithField {
                id: adjExposure;
                // Stops, so ±2 EV covers ordinary correction without the slider
                // becoming too coarse to be useful.
                from: -2; to: 2; value: 0; defaultValue: 0;
                unit: qsTr(" EV"); precision: 2;
                width: parent.width;
                onValueChanged: controller.set_color_adjustment("exposure", value);
            }
        }
        Label {
            text: qsTr("Contrast");
            SliderWithField {
                id: adjContrast;
                from: -1; to: 1; value: 0; defaultValue: 0;
                unit: "%"; precision: 0; scaler: 100.0;
                width: parent.width;
                onValueChanged: controller.set_color_adjustment("contrast", value);
            }
        }
        Label {
            text: qsTr("Saturation");
            SliderWithField {
                id: adjSaturation;
                from: -1; to: 1; value: 0; defaultValue: 0;
                unit: "%"; precision: 0; scaler: 100.0;
                width: parent.width;
                onValueChanged: controller.set_color_adjustment("saturation", value);
            }
        }
        Label {
            text: qsTr("Temperature");
            SliderWithField {
                id: adjTemperature;
                from: -1; to: 1; value: 0; defaultValue: 0;
                unit: "%"; precision: 0; scaler: 100.0;
                width: parent.width;
                onValueChanged: controller.set_color_adjustment("temperature", value);
            }
        }
        Label {
            text: qsTr("Tint");
            SliderWithField {
                id: adjTint;
                from: -1; to: 1; value: 0; defaultValue: 0;
                unit: "%"; precision: 0; scaler: 100.0;
                width: parent.width;
                onValueChanged: controller.set_color_adjustment("tint", value);
            }
        }
        Label {
            text: qsTr("Highlights");
            SliderWithField {
                id: adjHighlights;
                from: -1; to: 1; value: 0; defaultValue: 0;
                unit: "%"; precision: 0; scaler: 100.0;
                width: parent.width;
                onValueChanged: controller.set_color_adjustment("highlights", value);
            }
        }
        Label {
            text: qsTr("Shadows");
            SliderWithField {
                id: adjShadows;
                from: -1; to: 1; value: 0; defaultValue: 0;
                unit: "%"; precision: 0; scaler: 100.0;
                width: parent.width;
                onValueChanged: controller.set_color_adjustment("shadows", value);
            }
        }
        Label {
            text: qsTr("Vignette");
            SliderWithField {
                // One-sided: a negative vignette would brighten the corners,
                // which is not what the control means.
                id: adjVignette;
                from: 0; to: 1; value: 0; defaultValue: 0;
                unit: "%"; precision: 0; scaler: 100.0;
                width: parent.width;
                onValueChanged: controller.set_color_adjustment("vignette", value);
            }
        }

        Button {
            text: qsTr("Reset adjustments");
            iconName: "undo";
            width: parent.width;
            onClicked: root.resetAdjustments();
        }
    }
}
