// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

import QtQuick

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
    function getSettings(): var {
        const url = controller.get_color_lut_url();
        return url? { "url": url } : ({ });
    }

    /// Restores from a project. Missing or empty keys leave the panel untouched,
    /// so older projects load unchanged.
    function loadGyroflow(obj: var): void {
        const o = obj.color_lut;
        if (o && o.url) controller.load_color_lut_url(o.url);
    }

    FileDialog {
        id: lutFileDialog;
        title: qsTr("Choose a LUT file");
        nameFilters: [qsTr("LUT files") + " (*.cube *.CUBE)"];
        type: "lut";
        onAccepted: controller.load_color_lut(lutFileDialog.selectedFile);
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

    Button {
        text: qsTr("Remove LUT");
        iconName: "bin";
        width: parent.width;
        visible: root.hasLut;
        onClicked: controller.clear_color_lut();
    }
}
