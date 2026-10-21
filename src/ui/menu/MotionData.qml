// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2021-2022 Adrian <adrian.eddy at gmail>

import QtQuick
import QtQuick.Controls as QQC
import QtQuick.Dialogs

import "../components/"

MenuItem {
    id: root;
    text: qsTr("Motion data");
    iconName: "chart";
    loader: controller.loading_gyro_in_progress || root.gyrogateBusy;
    objectName: "motiondata";

    property alias hasQuaternions: integrator.hasQuaternions;
    property bool hasAccurateTimestamps: false;
    property bool hasFocusDistance: false;
    property bool hasIris: false;
    property alias hasRawGyro: integrator.hasRawGyro;
    property alias integrationMethod: integrator.currentIndex;
    property alias orientationIndicator: orientationIndicator;
    property bool allMetadata: allMetadataCb.visible && allMetadataCb.checked;
    property string filename: "";
    property string detectedFormat: "";
    property url lastSelectedFile: "";
    // DJI O4 Gyrogate detection of the loaded motion data (see `gyrogate` in controller.rs),
    // null for other cameras.
    property var gyrogate: null;
    property bool gyrogateBusy: false;
    property real gyrogateProgress: 0;
    // The clip the corrected copy is being written for, to apply it only if still loaded.
    property string gyrogateCopyFor: "";
    readonly property bool gyrogateRecommended: !!gyrogate && !!gyrogate.summary && (gyrogate.summary.bursts > 0 || gyrogate.summary.tremor_seconds > 0)
                                                && gyrogate.unit.chip != "mp66" && !gyrogate.repaired_copy;

    FileDialog {
        id: fileDialog;
        property var extensions: [ "csv", "txt", "bbl", "bfl", "mp4", "mov", "mxf", "insv", "gcsv", "360", "log", "bin", "braw", "r3d", "nev", "gpmf", "crm", "zraw", "jsonl" ];

        title: qsTr("Choose a motion data file")
        nameFilters: Qt.platform.os == "android"? undefined : [qsTr("Motion data files") + " (*." + extensions.concat(extensions.map(x => x.toUpperCase())).join(" *.") + ")"];
        type: "video";
        onAccepted: loadFile(selectedFile);
    }
    function loadFile(url: url): void {
        if (!window.videoArea.vid.loaded) {
            messageBox(Modal.Error, qsTr("Video file is not loaded."), [ { text: qsTr("Ok"), accent: true } ]);
            return;
        }
        lastSelectedFile = url;
        controller.load_telemetry(url, root.allMetadata, window.videoArea.vid, currentLog.visible && currentLog.currentIndex > 0? currentLog.currentIndex - 1 : -1, 0);
    }

    function loadGyroflow(obj: var): void {
        const gyro = obj.gyro_source || { };
        if (gyro && Object.keys(gyro).length > 0) {
            if (gyro.rotation && gyro.rotation.length == 3) {
                p.value = gyro.rotation[0];
                r.value = gyro.rotation[1];
                y.value = gyro.rotation[2];
                rot.checked = Math.abs(p.value) > 0 || Math.abs(r.value) > 0 || Math.abs(y.value) > 0;
            }
            if (gyro.acc_rotation && gyro.acc_rotation.length == 3) {
                ap.value = gyro.acc_rotation[0];
                ar.value = gyro.acc_rotation[1];
                ay.value = gyro.acc_rotation[2];
                arot.checked = Math.abs(ap.value) > 0 || Math.abs(ar.value) > 0 || Math.abs(ay.value) > 0;
                arot_action.checked = arot.checked;
            }
            if (gyro.imu_orientation) orientation.text = gyro.imu_orientation;
            if (gyro.hasOwnProperty("integration_method")) {
                const index = +gyro.integration_method;
                integrator.currentIndex = integrator.hasQuaternions? index : index - 1;
            }
            if (+gyro.lpf > 0) {
                lpf.value = +gyro.lpf;
                lpfcb.checked = lpf.value > 0;
            }
            if (typeof gyro.glitch_strength === "number" && +gyro.glitch_strength > 0) {
                glitchStrength.value = +gyro.glitch_strength;
            }
            if (gyro.hasOwnProperty("glitch_filter")) {
                glitchcb.checked = !!gyro.glitch_filter;
            }
            if (typeof gyro.gyrogate_strength === "number") {
                gyrogateStrength.value = +gyro.gyrogate_strength;
            }
            if (gyro.hasOwnProperty("gyrogate_filter")) {
                gyrogatecb.checked = !!gyro.gyrogate_filter;
            } else if (obj.videofile) {
                // A project saved before the repair existed keeps the look it was saved with.
                gyrogatecb.checked = false;
            }
            controller.set_gyrogate_filter(gyrogatecb.checked, gyrogateStrength.value);
            if (typeof gyro.sample_index === "number") {
                currentLog.currentIndex = gyro.sample_index + 1;
            }
        }
        const stab = obj.stabilization || { };
        if (stab && Object.keys(stab).length > 0) {
            focb.checked = +stab.frame_offset > 0;
            fo.value = +stab.frame_offset;
        }
    }
    function setGyroLpf(v: real): void {
        lpf.value = v;
        lpfcb.checked = +v > 0;
    }

    function msToTime(ms: real): string {
        if (ms >= 60*60*1000) {
            return new Date(ms).toISOString().substring(11, 11+8);
        } else {
            return new Date(ms).toISOString().substring(11+3, 11+8);
        }
    }
    Connections {
        target: controller;
        function onTelemetry_loaded(is_main_video: bool, filename: string, camera: string, additional_data: var): void {
            root.filename = filename || "";
            root.detectedFormat = camera || "";
            info.updateEntry("File name", filename || "---");
            info.updateEntry("Detected format", camera || "---");
            orientation.text = additional_data.imu_orientation;

            // Twice to trigger change signal
            integrator.hasRawGyro = additional_data.contains_raw_gyro;
            integrator.hasQuaternions = !additional_data.contains_quats;
            integrator.hasQuaternions = additional_data.contains_quats;
            root.hasAccurateTimestamps = additional_data.has_accurate_timestamps || false;
            root.hasFocusDistance = additional_data.contains_focus_distance || false;
            root.hasIris          = additional_data.contains_iris || false;
            if (additional_data.contains_quats && !is_main_video) {
                if (integrator.hasRawGyro) {
                    integrator.currentIndex = 2;
                } else {
                    integrator.currentIndex = 0;
                }
                integrateTimer.start();
            }
            if (!additional_data.contains_quats) {
                integrator.currentIndex = 1; // Default to VQF
            }

            controller.set_imu_lpf(lpfcb.checked? lpf.value : 0);
            controller.set_imu_median_filter(mfcb.checked? mf.value : 0);
            controller.set_glitch_filter(glitchcb.checked, glitchStrength.value);
            // On by default where the clip shows the defect; a project loaded afterwards
            // restores its own choice through loadGyroflow().
            root.gyrogate = additional_data.gyrogate || null;
            gyrogatecb.checked = root.gyrogateRecommended;
            controller.set_gyrogate_filter(gyrogatecb.checked, gyrogateStrength.value);
            // Show the user that the clip has the defect and that the repair can be tuned.
            // Not saved: the panel's own open/closed setting stays the user's choice.
            if (is_main_video && root.gyrogateRecommended && !root.opened) {
                root.opened = true;
                gyrogateRevealTimer.start();
            }
            controller.set_imu_rotation(rot.checked? p.value : 0, rot.checked? r.value : 0, rot.checked? y.value : 0);
            controller.set_acc_rotation(arot.checked? ap.value : 0, arot.checked? ar.value : 0, arot.checked? ay.value : 0);
            Qt.callLater(controller.recompute_gyro);

            Qt.callLater(window.videoArea.timeline.updateDurations);

            currentLog.preventChange = true;
            if (additional_data.usable_logs && additional_data.usable_logs.length > 0) {
                let model = ["All logs combined"];
                for (const log of additional_data.usable_logs) {
                    const [logIndex, startTimestamp, duration] = log.split(";");
                    model.push("#" + (+logIndex + 1) + " | " + msToTime(+startTimestamp) + " - " + msToTime(+startTimestamp + (+duration)) + " (" + msToTime(+duration) + ")");
                }
                if (currentLog.model != model)
                    currentLog.model = model;
            } else {
                currentLog.model = [];
            }
            currentLog.preventChange = false;
        }
        function onBias_estimated(biasX: real, biasY: real, biasZ: real): void {
            gyrobias.checked = true;
            bx.value = biasX;
            by.value = biasY;
            bz.value = biasZ;
        }
        function onOrientation_guessed(value: string): void {
             orientation.text = value;
        }
        function onChart_data_changed(): void {
            Qt.callLater(orientationIndicator.requestPaint);
        }
    }

    Button {
        text: qsTr("Open file");
        iconName: "file-empty"
        anchors.horizontalCenter: parent.horizontalCenter;
        onClicked: fileDialog.open2();
    }
    InfoMessageSmall {
        show: Qt.platform.os == "android" && !root.detectedFormat && root.lastSelectedFile.toString();
        type: InfoMessage.Info;
        text: qsTr("In order to detect multiple motion data files, click here and grant access to the directory with files.");
        OutputPathField { id: opf; visible: false; }
        MouseArea {
            anchors.fill: parent;
            cursorShape: Qt.PointingHandCursor;
            onClicked: {
                opf.selectFolder("", function(_) {
                    root.loadFile(root.lastSelectedFile);
                });
            }
        }
    }

    TableList {
        id: info;

        Component.onCompleted: {
            QT_TRANSLATE_NOOP("TableList", "File name"),
            QT_TRANSLATE_NOOP("TableList", "Detected format")
        }

        model: ({
            "File name": "---",
            "Detected format": "---"
        })
    }
    Label {
        position: Label.LeftPosition;
        text: qsTr("Select log");
        visible: currentLog.count > 1;

        ComboBox {
            id: currentLog;
            property bool preventChange: false;
            model: [QT_TRANSLATE_NOOP("Popup", "All logs combined")];
            font.pixelSize: 12 * dpiScale;
            width: parent.width;
            onCurrentIndexChanged: {
                if (!preventChange && count > 1) {
                    root.loadFile(root.lastSelectedFile);
                }
            }
        }
    }
    CheckBox {
        id: allMetadataCb;
        text: qsTr("Load all metadata");
        visible: root.lastSelectedFile.toString() && root.lastSelectedFile != window.videoArea.loadedFileUrl;
        checked: false;
        onCheckedChanged: allMetadataDebounce.start();
        Timer {
            id: allMetadataDebounce;
            interval: 1;
            running: false;
            repeat: false;
            onTriggered: root.loadFile(root.lastSelectedFile);
        }
    }
    CheckBoxWithContent {
        id: focb;
        text: qsTr("Frame offset");
        visible: allMetadataCb.visible;
        onCheckedChanged: {
            controller.frame_offset = focb.checked? fo.value : 0;
        }
        NumberField {
            id: fo;
            unit: qsTr("frames");
            precision: 0;
            value: 0;
            from: -100000;
            to: 100000;
            width: parent.width;
            tooltip: qsTr("Add or subtract frames from the video to align with motion data");
            onValueChanged: {
                controller.frame_offset = focb.checked? fo.value : 0;
            }
        }
    }
    CheckBoxWithContent {
        id: lpfcb;
        text: qsTr("Low pass filter");
        onCheckedChanged: {
            controller.set_imu_lpf(checked? lpf.value : 0);
            Qt.callLater(controller.recompute_gyro);
        }

        NumberField {
            id: lpf;
            unit: qsTr("Hz");
            precision: 2;
            value: 50;
            from: 0;
            width: parent.width;
            tooltip: qsTr("Lower cutoff frequency means more filtering");
            onValueChanged: {
                controller.set_imu_lpf(lpfcb.checked? value : 0);
                Qt.callLater(controller.recompute_gyro);
            }
        }
    }
    CheckBoxWithContent {
        id: mfcb;
        text: qsTr("Median filter");
        onCheckedChanged: {
            controller.set_imu_median_filter(checked? mf.value : 0);
            Qt.callLater(controller.recompute_gyro);
        }

        NumberField {
            id: mf;
            unit: qsTr("samples");
            precision: 0;
            value: 5;
            from: 0;
            width: parent.width;
            onValueChanged: {
                controller.set_imu_median_filter(mfcb.checked? value : 0);
                Qt.callLater(controller.recompute_gyro);
            }
        }
    }
    CheckBoxWithContent {
        id: glitchcb;
        text: qsTr("Glitch filtering");
        tooltip: qsTr("Detect and repair short bursts of corrupt gyro data");
        onCheckedChanged: {
            controller.set_glitch_filter(checked, glitchStrength.value);
            Qt.callLater(controller.recompute_gyro);
        }
        Label {
            text: qsTr("Strength");
            width: parent.width;
            tooltip: qsTr("Higher values detect glitches more aggressively (catching weaker and longer bursts with more passes), but may affect real fast motion. Lower values only repair the obvious, large glitches. 50% is the default.");
            SliderWithField {
                id: glitchStrength;
                defaultValue: 50;
                to: 100;
                value: 50;
                unit: "%";
                precision: 0;
                width: parent.width;
                onValueChanged: {
                    controller.set_glitch_filter(glitchcb.checked, value);
                    Qt.callLater(controller.recompute_gyro);
                }
            }
        }
    }
    Timer {
        id: gyrogateRevealTimer;
        interval: 800; // After the panel's opening animation.
        onTriggered: root.ensureVisible();
    }
    Column {
        id: gyrogateSection;
        width: parent.width;
        spacing: 5 * dpiScale;
        visible: !!root.gyrogate;

        function unitText(): string {
            const g = root.gyrogate;
            if (!g) return "";
            const built = g.unit.built? (("0" + g.unit.built[1]).slice(-2) + "/" + g.unit.built[0]) : "";
            switch (g.unit.chip) {
                case "icm40609d":         return qsTr("DJI O4 Lite: ICM-40609-D gyro, which records false rotations under vibration (DJI Gyrogate).");
                case "icm40609d_likely":  return qsTr("DJI O4 Pro built in %1: most likely the new ICM-40609-D gyro, which records false rotations under vibration (DJI Gyrogate).").arg(built);
                case "unsure":            return qsTr("DJI O4 Pro built in %1, when DJI was switching to the ICM-40609-D gyro: it may be affected by DJI Gyrogate.").arg(built);
                case "mp66":              return qsTr("DJI O4 Pro built in %1: original MP66 gyro, not affected by DJI Gyrogate.").arg(built);
                default:                  return qsTr("DJI O4 Pro with an unknown production date: it may be affected by DJI Gyrogate.");
            }
        }
        function summaryText(): string {
            const g = root.gyrogate;
            if (!g) return "";
            if (g.repaired_copy) return qsTr("This is a corrected copy: its false rotations were already removed.");
            if (!g.summary) return qsTr("No 1 kHz attitude data in this clip. Record with RockSteady and HorizonSteady off to be able to repair it.");
            const tremor = g.summary.tremor_seconds > 0? qsTr("Dense single-axis tremor in %1 s of the clip.").arg(g.summary.tremor_seconds.toFixed(1)) : "";
            if (g.summary.bursts == 0) return tremor || qsTr("No false rotations found in this clip.");
            const bursts = g.unit.chip == "mp66"
                ? qsTr("%n short single-axis jolt(s) found. With the original MP66 gyro these are usually real knocks, such as setting the drone down, so the repair is off by default.", "", g.summary.bursts)
                : qsTr("%n false rotation(s) found in this clip, %1° in total, the largest %2°.", "", g.summary.bursts)
                      .arg(g.summary.total_deg.toFixed(1)).arg(g.summary.largest_deg.toFixed(1));
            return tremor? bursts + " " + tremor : bursts;
        }

        InfoMessageSmall {
            show: gyrogateSection.visible;
            type: root.gyrogateRecommended? InfoMessage.Warning : InfoMessage.Info;
            text: gyrogateSection.unitText() + "\n" + gyrogateSection.summaryText();
        }
        CheckBoxWithContent {
            id: gyrogatecb;
            text: qsTr("Remove DJI Gyrogate false rotations");
            tooltip: qsTr("Removes the short false rotations recorded by the ICM-40609-D gyro of DJI O4 units and the slow drift the camera adds while pulling them back, and smooths the dense single-axis tremor the same gyro records. Clips without either are left untouched.");
            onCheckedChanged: {
                controller.set_gyrogate_filter(checked, gyrogateStrength.value);
                Qt.callLater(controller.recompute_gyro);
            }
            Column {
                id: gyrogateSensitivity;
                width: parent.width;
                spacing: 4 * dpiScale;
                readonly property string help: qsTr("How small a false rotation can be and still be removed. A rotation counts as false when one axis jumps above the threshold for a few milliseconds while the other two stay calm.") + "\n\n" +
                         qsTr("0%: threshold 120 °/s, only the large, obvious jolts.\n50% (default): threshold 60 °/s, tuned against Betaflight blackbox logs.\n100%: threshold 30 °/s, also small jolts, but it may mistake real knocks for false ones.") + "\n\n" +
                         qsTr("Raise it if small jolts remain in the stabilized video with the repair on. Lower it if real motion, such as a knock or an impact, looks smoothed away. The value is saved with the project and used for the corrected copy.") + "\n\n" +
                         qsTr("The same setting moves the tremor floor, a quarter of the threshold (15 °/s at 50%). The tremor is smoothed on its axis only, where small spikes pile up, many per second.");
                Row {
                    spacing: 2 * dpiScale;
                    BasicText {
                        leftPadding: 0;
                        text: qsTr("Sensitivity");
                        anchors.verticalCenter: parent.verticalCenter;
                    }
                    LinkButton {
                        iconName: "question";
                        display: QQC.Button.IconOnly;
                        leftPadding: 3 * dpiScale;
                        rightPadding: 3 * dpiScale;
                        anchors.verticalCenter: parent.verticalCenter;
                        tooltip: gyrogateSensitivity.help;
                        // A click shows the same text, for touch screens without hover.
                        onClicked: messageBox(Modal.Info, gyrogateSensitivity.help, [ { text: qsTr("Ok") } ]);
                    }
                }
                SliderWithField {
                    id: gyrogateStrength;
                    defaultValue: 50;
                    to: 100;
                    value: 50;
                    unit: "%";
                    precision: 0;
                    width: parent.width;
                    onValueChanged: {
                        controller.set_gyrogate_filter(gyrogatecb.checked, value);
                        Qt.callLater(controller.recompute_gyro);
                    }
                }
            }
        }
        Button {
            text: qsTr("Save corrected copy");
            iconName: "save";
            // Android only hands over content URIs, not a folder to write next to the clip.
            visible: Qt.platform.os != "android";
            enabled: !root.gyrogateBusy && !!root.gyrogate && !!root.gyrogate.summary && !root.gyrogate.repaired_copy;
            tooltip: qsTr("Writes a copy of the clip with the false rotations removed next to the original (%1), and uses it as the motion data. The video and audio are not changed and the original is kept.").arg("_gyrofix");
            anchors.horizontalCenter: parent.horizontalCenter;
            onClicked: {
                root.gyrogateBusy = true;
                root.gyrogateProgress = 0;
                root.gyrogateCopyFor = window.videoArea.loadedFileUrl.toString() + "|" + root.filename;
                controller.save_gyrogate_copy(gyrogateStrength.value);
            }
        }
        BasicText {
            visible: root.gyrogateBusy;
            width: parent.width;
            horizontalAlignment: Text.AlignHCenter;
            text: qsTr("Saving the corrected copy... %1%").arg(Math.round(root.gyrogateProgress * 100));
        }
        Connections {
            target: controller;
            function onGyrogate_copy_progress(percent: real, error_string: string, url: string, report: string): void {
                root.gyrogateProgress = percent;
                if (percent < 1) return;
                root.gyrogateBusy = false;
                if (error_string) {
                    messageBox(Modal.Error, qsTr("Could not save the corrected copy: %1").arg(error_string), [ { text: qsTr("Ok") } ]);
                    return;
                }
                if (!url) {
                    messageBox(Modal.Info, qsTr("No false rotations were found at this sensitivity, so no copy was written."), [ { text: qsTr("Ok") } ]);
                    return;
                }
                const r = JSON.parse(report || "{}");
                const tremor = r.tremor_seconds > 0? " " + qsTr("Tremor smoothed in %1 s.").arg(r.tremor_seconds.toFixed(1)) : "";
                window.showNotification(Modal.Info, qsTr("Corrected copy saved to %1 (%n false rotation(s) removed).", "", (r.bursts || []).length).arg("<b>" + filesystem.display_url(url) + "</b>") + tremor);
                if (root.gyrogateCopyFor != window.videoArea.loadedFileUrl.toString() + "|" + root.filename) {
                    return; // Another clip was opened meanwhile.
                }
                // Use the copy as the motion data and keep everything else: the current
                // project is reapplied with only the motion data file swapped.
                const obj = JSON.parse(controller.export_gyroflow_data("Simple", window.getAdditionalProjectData()));
                obj.gyro_source = obj.gyro_source || { };
                obj.gyro_source.filepath = url;
                // The macOS bookmark and the metadata still describe the original file and
                // would take precedence over `filepath` on import.
                delete obj.gyro_source.filepath_bookmark;
                delete obj.gyro_source.file_metadata;
                obj.gyro_source.gyrogate_filter = false;
                window.videoArea.loadGyroflowData(obj, render_queue.editing_job_id);
            }
        }
    }
    Item {
        width: parent.width;
        height: rot.height;
        CheckBoxWithContent {
            id: rot;
            text: qsTr("Rotation");
            onCheckedChanged: update_rotation();
            function update_rotation(): void {
                controller.set_imu_rotation(rot.checked? p.value : 0, rot.checked? r.value : 0, rot.checked? y.value : 0);
                Qt.callLater(controller.recompute_gyro);
            }
            ContextMenuMouseArea {
                parent: rot.cb;
                cursorShape: Qt.PointingHandCursor;
                onContextMenu: (isHold, x, y) => { contextMenu.popup(rot, x, y); }
            }

            Flow {
                width: parent.width;
                spacing: 5 * dpiScale;
                Label {
                    position: Label.LeftPosition;
                    text: qsTr("Pitch");
                    width: undefined;
                    inner.width: 50 * dpiScale;
                    spacing: 5 * dpiScale;
                    NumberField { id: p; unit: "°"; precision: 1; from: -360; to: 360; width: 50 * dpiScale; onValueChanged: rot.update_rotation(); tooltip: qsTr("Pitch is camera angle up/down when using FPV blackbox data"); }
                }
                Label {
                    position: Label.LeftPosition;
                    text: qsTr("Roll");
                    width: undefined;
                    inner.width: 50 * dpiScale;
                    spacing: 5 * dpiScale;
                    NumberField { id: r; unit: "°"; precision: 1; from: -360; to: 360; width: 50 * dpiScale; onValueChanged: rot.update_rotation(); }
                }
                Label {
                    position: Label.LeftPosition;
                    text: qsTr("Yaw");
                    width: undefined;
                    inner.width: 50 * dpiScale;
                    spacing: 5 * dpiScale;
                    NumberField { id: y; unit: "°"; precision: 1; from: -360; to: 360; width: 50 * dpiScale; onValueChanged: rot.update_rotation(); }
                }
            }
        }
        Menu {
            id: contextMenu;
            font.pixelSize: 11.5 * dpiScale;
            Action {
                id: arot_action;
                iconName: "axes";
                text: qsTr("Separate accelerometer rotation");
                checkable: true;
            }
        }
    }
    CheckBoxWithContent {
        id: arot;
        visible: arot_action.checked;
        text: qsTr("Accelerometer rotation");
        onCheckedChanged: update_rotation();
        function update_rotation(): void {
            controller.set_acc_rotation(arot.checked? ap.value : 0, arot.checked? ar.value : 0, arot.checked? ay.value : 0);
            Qt.callLater(controller.recompute_gyro);
        }

        Flow {
            width: parent.width;
            spacing: 5 * dpiScale;
            Label {
                position: Label.LeftPosition;
                text: qsTr("Pitch");
                width: undefined;
                inner.width: 50 * dpiScale;
                spacing: 5 * dpiScale;
                NumberField { id: ap; unit: "°"; precision: 1; from: -360; to: 360; width: 50 * dpiScale; onValueChanged: arot.update_rotation(); }
            }
            Label {
                position: Label.LeftPosition;
                text: qsTr("Roll");
                width: undefined;
                inner.width: 50 * dpiScale;
                spacing: 5 * dpiScale;
                NumberField { id: ar; unit: "°"; precision: 1; from: -360; to: 360; width: 50 * dpiScale; onValueChanged: arot.update_rotation(); }
            }
            Label {
                position: Label.LeftPosition;
                text: qsTr("Yaw");
                width: undefined;
                inner.width: 50 * dpiScale;
                spacing: 5 * dpiScale;
                NumberField { id: ay; unit: "°"; precision: 1; from: -360; to: 360; width: 50 * dpiScale; onValueChanged: arot.update_rotation(); }
            }
        }
    }
    CheckBoxWithContent {
        id: gyrobias;
        text: qsTr("Gyro bias");
        onCheckedChanged: update_bias();
        function update_bias(): void {
            controller.set_imu_bias(gyrobias.checked? bx.value : 0, gyrobias.checked? by.value : 0, gyrobias.checked? bz.value : 0);
            Qt.callLater(controller.recompute_gyro);
        }

        Flow {
            width: parent.width;
            spacing: 5 * dpiScale;
            Label {
                position: Label.LeftPosition;
                text: qsTr("X");
                width: undefined;
                inner.width: 65 * dpiScale;
                spacing: 5 * dpiScale;
                NumberField { id: bx; unit: "°/s"; precision: 2; width: 65 * dpiScale; onValueChanged: gyrobias.update_bias(); }
            }
            Label {
                position: Label.LeftPosition;
                text: qsTr("Y");
                width: undefined;
                inner.width: 65 * dpiScale;
                spacing: 5 * dpiScale;
                NumberField { id: by; unit: "°/s"; precision: 2; width: 65 * dpiScale; onValueChanged: gyrobias.update_bias(); }
            }
            Label {
                position: Label.LeftPosition;
                text: qsTr("Z");
                width: undefined;
                inner.width: 65 * dpiScale;
                spacing: 5 * dpiScale;
                NumberField { id: bz; unit: "°/s"; precision: 2; width: 65 * dpiScale; onValueChanged: gyrobias.update_bias(); }
            }
        }
    }
    Label {
        position: Label.LeftPosition;
        text: qsTr("IMU orientation");

        TextField {
            id: orientation;
            width: parent.width;
            text: "XYZ";
            validator: RegularExpressionValidator { regularExpression: /[XYZxyz]{3}/; }
            tooltip: qsTr("Uppercase is positive, lowercase is negative. eg. zYX");
            onTextChanged: if (acceptableInput) { controller.set_imu_orientation(text); Qt.callLater(controller.recompute_gyro); }
        }
    }
    Label {
        position: Label.LeftPosition;
        text: qsTr("Integration method");

        ComboBox {
            id: integrator;
            property bool hasQuaternions: false;
            property bool hasRawGyro: false;
            model: hasQuaternions? [QT_TRANSLATE_NOOP("Popup", "None"), "Complementary", "VQF", "Simple gyro", "Simple gyro + accel", "Mahony", "Madgwick" ] : ["Complementary", "VQF", "Simple gyro", "Simple gyro + accel", "Mahony", "Madgwick"];
            font.pixelSize: 12 * dpiScale;
            width: parent.width;
            tooltip: hasQuaternions && currentIndex === 0? qsTr("Use built-in quaternions instead of IMU data") : qsTr("IMU integration method for calculating motion data");
            function setMethod(): void {
                controller.set_integration_method(hasQuaternions? currentIndex : currentIndex + 1);
            }
            onCurrentIndexChanged: integrateTimer.start();
            onHasQuaternionsChanged: integrateTimer.start();
            Timer {
                id: integrateTimer;
                interval: 300;
                onTriggered: Qt.callLater(integrator.setMethod);
            }
        }
    }

    CheckBoxWithContent {
        id: orientationCheckbox;
        text: qsTr("Orientation indicator");
        onCheckedChanged: Qt.callLater(orientationIndicator.requestPaint);

        Canvas {
            id: orientationIndicator
            width: parent.width
            height: 100
            property real currentTimestamp: 0
            property bool initialDraw: false
            onPaint: {
                if (orientationCheckbox.checked || !initialDraw) {
                    initialDraw = true
                    let ctx = getContext("2d");
                    ctx.reset();
                    const veclen = 30;
                    const xv = Qt.vector3d(0,veclen,0)
                    const yv = Qt.vector3d(-veclen,0,0)
                    const zv = Qt.vector3d(0,0,veclen)
                    const vecs = [xv, yv, zv]
                    const colors = style === "light" ? ['#cc0000', '#00cc00', '#0000cc'] : ['#ff0000', '#00ff00', '#4444ff'];
                    // inspired by blender camera
                    const cam_width = 30;
                    const cam_height = 15;
                    const cam_length = 30;
                    const cam_vertices = [[-cam_width,-cam_height,-cam_length],
                                          [cam_width, -cam_height,-cam_length],
                                          [cam_width, cam_height, -cam_length],
                                          [-cam_width, cam_height, -cam_length],
                                          [0,0,0]]
                    const cam_vert_vecs = cam_vertices.map(vert => Qt.vector3d(vert[0],vert[1],vert[2]))
                    const lines = [[0,1,2,3,0],
                                   [0,4,1],
                                   [2,4,3]]

                    const quats = controller.quats_at_timestamp(Math.round(currentTimestamp))
                    const transform = Qt.quaternion( quats[0], quats[1],  quats[2], quats[3]); // wxyz
                    const maincolor = style === "light" ? "rgba(0,0,0,0.9)" : "rgba(255,255,255,0.9)";
                    const transform_smooth = transform.times(Qt.quaternion( quats[4], quats[5],  quats[6], quats[7]).inverted());
                    const transforms = [transform, transform_smooth]

                    // center dots
                    for (let i = 0; i < 3; i++) {
                        ctx.beginPath();
                        ctx.arc(width/6*(i*2+1), height/2, 4, 0, 2 * Math.PI, false);
                        ctx.fillStyle = maincolor;
                        ctx.fill();
                        ctx.stroke();
                    }

                    for (let i = 0; i < 3; i++) {
                        ctx.beginPath();
                        ctx.moveTo(width/6, height/2);
                        const transformedvec = transform.times(vecs[i])
                        ctx.lineTo(width/6 + transformedvec.x, height/2 - transformedvec.y);
                        ctx.lineWidth = 3;
                        ctx.strokeStyle = colors[i];
                        ctx.globalAlpha = 0.5;
                        ctx.stroke();
                        ctx.globalAlpha = Math.max(0.1, Math.min(transformedvec.z/(veclen*2)+0.5,1));
                        ctx.beginPath();
                        ctx.arc(width/6 + transformedvec.x, height/2 - transformedvec.y, 4, 0, 2 * Math.PI, false);
                        ctx.fillStyle = colors[i];
                        ctx.fill();
                        ctx.stroke();
                    }

                    ctx.lineWidth = 1.5;
                    ctx.strokeStyle = maincolor;
                    ctx.globalAlpha = 0.8;
                    ctx.lineJoin = "bevel";
                    for (let view = 0; view < 2; view++) {
                        for (let linenum = 0; linenum < lines.length; linenum++) {
                            ctx.beginPath()
                            for (let pointnum=0; pointnum < lines[linenum].length; pointnum++) {
                                const transformedvec = transforms[view].times(cam_vert_vecs[lines[linenum][pointnum]]);
                                if (pointnum == 0) {
                                    ctx.moveTo(transformedvec.x + width/6*(view*2 + 3), -transformedvec.y + height/2);
                                }
                                else {
                                    ctx.lineTo(transformedvec.x + width/6*(view*2 + 3), -transformedvec.y + height/2);
                                }
                            }
                            ctx.stroke();
                        }
                    }
                }
            }
            function updateOrientation(timestamp: real): void {
                currentTimestamp = timestamp;
                requestPaint();
                meshCorrection.requestPaint();
                focalPlaneDistortion.requestPaint();
            }
        }
        Canvas {
            id: meshCorrection
            width: parent.width
            height: width * (orgH / orgW);
            visible: false;

            property real orgW: window.videoArea.outWidth  || window.videoArea.vid.videoWidth;
            property real orgH: window.videoArea.outHeight || window.videoArea.vid.videoHeight;
            property bool initialDraw: false
            onPaint: {
                if (orientationCheckbox.checked || !initialDraw) {
                    initialDraw = true
                    let ctx = getContext("2d");
                    ctx.reset();
                    const maincolor = style === "light" ? "rgba(0,0,0,0.9)" : "rgba(255,255,255,0.9)";
                    const margin = 15 * dpiScale;

                    const mesh = controller.mesh_at_frame(window.videoArea.vid.currentFrame);
                    if (!mesh.length || mesh[0] < 10) { meshCorrection.visible = false; return; }
                    const divisions = [mesh[1], mesh[2]];
                    const mesh_size = [mesh[3], mesh[4]];

                    meshCorrection.visible = divisions[0] > 0;

                    for (let i = 0; i < (divisions[0]*divisions[1]*2); i += 2) {
                        const x = margin + (width  - 2*margin) * mesh[9 + i + 0] / mesh_size[0];
                        const y = margin + (height - 2*margin) * mesh[9 + i + 1] / mesh_size[1];
                        ctx.beginPath();
                        ctx.arc(x, y, 2 * dpiScale, 0, 2 * Math.PI, false);
                        ctx.fillStyle = maincolor;
                        ctx.fill();
                    }

                    ctx.lineWidth = 1 * dpiScale;
                    ctx.strokeStyle = maincolor;
                    ctx.beginPath();
                    for (let i = 0; i < (divisions[0]*divisions[1]*2); i += 2) {
                        const x = margin + (width  - 2*margin) * (mesh[9 + i + 0] / mesh_size[0]);
                        const y = margin + (height - 2*margin) * (mesh[9 + i + 1] / mesh_size[1]);
                        ctx.moveTo(x, y);
                        if (((i + 2) / 2) % divisions[1] != 0) {
                            const xx = margin + (width  - 2*margin) * mesh[9 + i + 2 + 0] / mesh_size[0];
                            const yy = margin + (height - 2*margin) * mesh[9 + i + 2 + 1] / mesh_size[1];
                            ctx.lineTo(xx, yy);
                        }
                        if (i + divisions[1]*2 < divisions[0]*divisions[1]*2) {
                            const xxx = margin + (width  - 2*margin) * (mesh[9 + i + divisions[0]*2 + 0] / mesh_size[0]);
                            const yyy = margin + (height - 2*margin) * (mesh[9 + i + divisions[0]*2 + 1] / mesh_size[1]);
                            ctx.moveTo(x, y);
                            ctx.lineTo(xxx, yyy);
                        }
                    }
                    ctx.stroke();
                }
            }
        }
        Canvas {
            id: focalPlaneDistortion;
            width: parent.width
            height: width * (orgH / orgW);
            visible: false;

            property real orgW: window.videoArea.outWidth  || window.videoArea.vid.videoWidth;
            property real orgH: window.videoArea.outHeight || window.videoArea.vid.videoHeight;
            property bool initialDraw: false
            onPaint: {
                if (orientationCheckbox.checked || !initialDraw) {
                    initialDraw = true
                    let ctx = getContext("2d");
                    ctx.reset();
                    const maincolor = style === "light" ? "rgba(0,0,0,0.9)" : "rgba(255,255,255,0.9)";
                    const margin = 15 * dpiScale;

                    const mesh = controller.mesh_at_frame(window.videoArea.vid.currentFrame);
                    if (!mesh.length || mesh[0] == 0 || mesh[mesh[0]] == 0) { focalPlaneDistortion.visible = false; return; }
                    const mesh_size = [mesh[3], mesh[4]];

                    focalPlaneDistortion.visible = true;

                    ctx.lineWidth = 1 * dpiScale;
                    ctx.strokeStyle = maincolor;
                    const o = mesh[0];
                    const stblz_grid = mesh_size[1] / 8;

                    let points = [];
                    for (let i = 0; i < 8; ++i) {
                        // corners of the rectangle
                        points.push([0, i * stblz_grid]);
                        points.push([mesh_size[0], i * stblz_grid]);
                        points.push([mesh_size[0], (i + 1) * stblz_grid]);
                        points.push([0, (i + 1) * stblz_grid]);

                        points.push([0, i * stblz_grid]);
                    }

                    for (let i = 0; i < points.length; ++i) {
                        const idx = Math.min(7, Math.max(0, Math.floor(points[i][1] / stblz_grid)));
                        const delta = points[i][1] - stblz_grid * idx;
                        points[i][0] += mesh[o + 4 + idx * 2 + 0] * delta;
                        points[i][1] += mesh[o + 4 + idx * 2 + 1] * delta;
                        for (let j = 0; j < idx; j++) {
                            points[i][0] += mesh[o + 4 + j * 2 + 0] * stblz_grid;
                            points[i][1] += mesh[o + 4 + j * 2 + 1] * stblz_grid;
                        }
                    }

                    ctx.beginPath();
                    for (let i = 0; i < points.length; ++i) {
                        const x = margin + (width  - 2*margin) * points[i][0] / mesh_size[0];
                        const y = margin + (height - 2*margin) * points[i][1] / mesh_size[1];
                        if (i == 0) ctx.moveTo(x, y);
                        else ctx.lineTo(x, y);
                    }
                    ctx.stroke();
                }
            }
        }
    }

    Row {
        anchors.horizontalCenter: parent.horizontalCenter;
        LinkButton {
            text: qsTr("Statistics");
            onClicked: {
                if (window.videoArea.statistics.item) window.videoArea.statistics.item.shown = !window.videoArea.statistics.item.shown;
                window.videoArea.statistics.active = true;
            }
        }
        LinkButton {
            id: exportGyroBtn;
            text: qsTr("Export");
            onClicked: {
                menuLoader.toggle(exportGyroBtn, 0, height);
            }
            Component {
                id: menu;
                Menu {
                    id: menuInner;
                    FileDialog {
                        id: exportFileDialog;
                        fileMode: FileDialog.SaveFile;
                        title: qsTr("Select file destination");
                        type: "gyro-csv";
                        property var exportData: ({});
                        onAccepted: {
                            if (exportData === "full") {
                                controller.export_full_metadata(selectedFile, root.lastSelectedFile.toString()? root.lastSelectedFile : window.videoArea.loadedFileUrl);
                            } else if (exportData == "parsed") {
                                controller.export_parsed_metadata(selectedFile);
                            } else {
                                controller.export_gyro_data(selectedFile, exportData);
                            }
                        }
                    }
                    Action {
                        text: qsTr("Export camera data (CSV/JSON/USD/AE)");
                        onTriggered: {
                            const el = Qt.createComponent("../SettingsSelector.qml").createObject(window, {
                                desc: [
                                    {
                                        "Original|original": {
                                            "Gyroscope":       ["gyroscope"],
                                            "Accelerometer":   ["accelerometer"],
                                            "Quaternion":      ["quaternion"],
                                            "Euler angles":    ["euler_angles"],
                                            "Focus distances": ["focus_distances"],
                                            "Iris (f/T-stop)": ["iris"],
                                        },
                                    },
                                    {
                                        "Stabilized|stabilized": {
                                            "Quaternion":    ["quaternion"],
                                            "Euler angles":  ["euler_angles"],
                                        },
                                    },
                                    {
                                        "Zooming|zooming": {
                                            "Minimal FOV scale":  ["minimal_fovs"],
                                            "Smoothed FOV scale": ["fovs"],
                                            "Focal length (if available)": ["focal_length"],
                                        },
                                    }
                                ],
                                type: "gyro_csv",
                                // Files with only built-in quaternions (eg. DJI) have no raw IMU data to export
                                unavailable: (integrator.hasRawGyro?    [] : ["originalgyroscope", "originalaccelerometer"])
                                     .concat(root.hasFocusDistance?     [] : ["originalfocus_distances"])
                                     .concat(root.hasIris?              [] : ["originaliris"])
                            });
                            let savedState = settings.value("CSVExportSelection", "");
                            if (savedState) {
                                try {
                                    el.loadSelection(JSON.parse(savedState));
                                } catch(e) { }
                            }
                            el.opened = true;
                            el.onApply.connect((obj) => {
                                settings.setValue("CSVExportSelection", JSON.stringify(obj));

                                if (Qt.platform.os == "ios") {
                                    const exportToFolder = ext => {
                                        const folder = filesystem.get_folder(root.lastSelectedFile.toString()? root.lastSelectedFile : window.videoArea.loadedFileUrl);
                                        const opf = Qt.createComponent("../components/OutputPathField.qml").createObject(window, { visible: false });
                                        opf.selectFolder(folder, function(folder_url) {
                                            const filename = root.filename.replace(/\.[^/.]+$/, "." + ext);
                                            controller.export_gyro_data(filesystem.get_file_url(folder_url, filename, true), obj);
                                            opf.destroy();
                                        });
                                    };
                                    messageBox(Modal.Question, qsTr("Which format do you want to use?"), [
                                        { text: "CSV",                         clicked: function() { exportToFolder("csv"); } },
                                        { text: "JSON", accent: true,          clicked: function() { exportToFolder("json"); } },
                                        { text: "Universal Scene Description", clicked: function() { exportToFolder("usd"); } },
                                        { text: "After Effects Script",        clicked: function() { exportToFolder("jsx"); } },
                                        { text: qsTr("Cancel") },
                                    ]);
                                    return;
                                }

                                exportFileDialog.nameFilters = ["CSV (*.csv)", "JSON (*.json)", "Universal Scene Description (*.usd)", "After Effects Script (*.jsx)"];
                                exportFileDialog.exportData = obj;
                                exportFileDialog.open2();
                            });
                        }
                    }
                    Action {
                        text: qsTr("Export full metadata");
                        onTriggered: {
                            if (Qt.platform.os == "ios") {
                                const folder = filesystem.get_folder(root.lastSelectedFile.toString()? root.lastSelectedFile : window.videoArea.loadedFileUrl);
                                const opf = Qt.createComponent("../components/OutputPathField.qml").createObject(window, { visible: false });
                                opf.selectFolder(folder, function(folder_url) {
                                    const filename = root.filename.replace(/\.[^/.]+$/, ".json");
                                    controller.export_full_metadata(filesystem.get_file_url(folder_url, filename, true), root.lastSelectedFile.toString()? root.lastSelectedFile : window.videoArea.loadedFileUrl);
                                    opf.destroy();
                                });
                                return;
                            }

                            exportFileDialog.nameFilters = ["JSON (*.json)"];
                            exportFileDialog.exportData = "full";
                            exportFileDialog.open2();
                        }
                    }
                    Action {
                        text: qsTr("Export parsed metadata");
                        onTriggered: {
                            if (Qt.platform.os == "ios") {
                                const folder = filesystem.get_folder(root.lastSelectedFile.toString()? root.lastSelectedFile : window.videoArea.loadedFileUrl);
                                const opf = Qt.createComponent("../components/OutputPathField.qml").createObject(window, { visible: false });
                                opf.selectFolder(folder, function(folder_url) {
                                    const filename = root.filename.replace(/\.[^/.]+$/, ".json");
                                    controller.export_parsed_metadata(filesystem.get_file_url(folder_url, filename, true));
                                    opf.destroy();
                                });
                                return;
                            }
                            exportFileDialog.nameFilters = ["JSON (*.json)"];
                            exportFileDialog.exportData = "parsed";
                            exportFileDialog.open2();
                        }
                    }
                    Action {
                        text: qsTr("Export project file (including processed gyro data)");
                        onTriggered: window.saveProject("WithProcessedData");
                    }
                }
            }
            ContextMenuLoader {
                id: menuLoader;
                sourceComponent: menu
            }
        }
    }

    DropTarget {
        parent: root.innerItem;
        color: styleBackground2;
        z: 999;
        anchors.rightMargin: -28 * dpiScale;
        anchors.topMargin: 35 * dpiScale;
        anchors.bottomMargin: -35 * dpiScale;
        extensions: fileDialog.extensions;
        onLoadFile: (url) => root.loadFile(url)
    }
}
