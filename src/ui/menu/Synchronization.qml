// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2021-2022 Adrian <adrian.eddy at gmail>

import QtQuick

import "../components/"

MenuItem {
    id: sync;
    text: qsTr("Synchronization");
    iconName: "sync";
    innerItem.enabled: window.videoArea.vid.loaded && !controller.sync_in_progress;
    loader: controller.sync_in_progress;
    objectName: "synchronization";

    Item {
        id: sett;
        property alias processingResolution: processingResolution.currentIndex;
        property alias initialOffset: initialOffset.value;
        property alias syncSearchSize: syncSearchSize.value;
        property alias maxSyncPoints: maxSyncPoints.value;
        property alias timePerSyncpoint: timePerSyncpoint.value;
        property alias sync_lpf: lpf.value;
        property alias checkNegativeInitialOffset: checkNegativeInitialOffset.checked;
        property alias experimentalAutoSyncPoints: experimentalAutoSyncPoints.checked;
        // property alias syncMethod: syncMethod.currentIndex;
        // property alias offsetMethod: offsetMethod.currentIndex;
        // property alias poseMethod: poseMethod.currentIndex;
        property alias showFeatures: showFeatures.checked;
        property alias showOF: showOF.checked;
        // This is a specific use case and I don't think we should remember that setting, especially that it's hidden under "Advanced"
        //property alias everyNthFrame: everyNthFrame.value;

        Component.onCompleted: settings.init(sett);
        function propChanged() { settings.propChanged(sett); }
    }

    property alias timePerSyncpoint: timePerSyncpoint;
    property alias everyNthFrame: everyNthFrame;
    property alias poseMethod: poseMethod;
    property var customSyncTimestamps: [];
    property var additionalSyncTimestamps: [];

    function loadGyroflow(obj: var): void {
        const o = obj.synchronization || { };
        if (o && Object.keys(o).length > 0) {
            if (o.hasOwnProperty("initial_offset"))     initialOffset.value                 = +o.initial_offset;
            if (o.hasOwnProperty("initial_offset_inv")) checkNegativeInitialOffset.checked  = !!o.initial_offset_inv;
            if (o.hasOwnProperty("search_size"))        syncSearchSize.value                = +o.search_size;
            if (o.hasOwnProperty("calc_initial_fast"))  calculateInitialOffsetFirst.checked = !!o.calc_initial_fast;
            if (o.hasOwnProperty("max_sync_points"))    maxSyncPoints.value                 = +o.max_sync_points;
            if (o.hasOwnProperty("every_nth_frame"))    everyNthFrame.value                 = +o.every_nth_frame;
            if (o.hasOwnProperty("time_per_syncpoint")) timePerSyncpoint.value              = +o.time_per_syncpoint;
            if (o.hasOwnProperty("of_method"))          syncMethod.currentIndex             = +o.of_method;
            if (o.hasOwnProperty("offset_method"))      offsetMethod.currentIndex           = +o.offset_method;
            if (o.hasOwnProperty("pose_method"))        poseMethod.currentIndex             = +o.pose_method;
            if (o.hasOwnProperty("custom_sync_pattern")) sync.customSyncTimestamps          = resolveSyncpointPattern(o.custom_sync_pattern);
            if (o.hasOwnProperty("auto_sync_points")) experimentalAutoSyncPoints.checked    = !!o.auto_sync_points;
            if (o.hasOwnProperty("do_autosync") && o.do_autosync) autosyncTimer.doRun = true;
        }

        // ---- Áudio externo ----
        // Chave separada de "synchronization" para não colidir com o sync óptico.
        // Projetos salvos antes desta feature simplesmente não têm a chave.
        const a = obj.audio_sync || { };
        if (a && a.hasOwnProperty("path") && a.path) {
            if (a.hasOwnProperty("preserve_original_format"))
                sync.audioPreserveFormat = !!a.preserve_original_format;
            if (a.hasOwnProperty("auto_band"))   sync.audioAutoBand  = !!a.auto_band;
            if (a.hasOwnProperty("band_lo_hz"))  sync.audioBandLo    = +a.band_lo_hz;
            if (a.hasOwnProperty("band_hi_hz"))  sync.audioBandHi    = +a.band_hi_hz;
            if (a.hasOwnProperty("highpass_hz")) sync.audioHighpass  = +a.highpass_hz;

            // O offset é aplicado depois do decode: recarregar o arquivo
            // redetecta canais/bit depth/float da origem, e só então faz sentido
            // reposicionar a waveform.
            const pendingOffset = a.hasOwnProperty("offset_seconds") ? +a.offset_seconds : 0.0;
            const waveform = window.videoArea.timeline.getAudioWaveform();
            if (controller.import_external_audio_url(a.path, waveform)) {
                audioOffset.value = pendingOffset * 1000.0;
            }
        }
    }

    /// Estado da trilha de áudio externa gravado no `.gyroflow`.
    function getAudioSyncSettings(): var {
        const path = controller.get_external_audio_url();
        if (!path) return { };
        return {
            "path":                     path,
            "sample_rate":              controller.get_external_audio_sample_rate(),
            "offset_seconds":           audioOffset.value / 1000.0,
            "preserve_original_format": sync.audioPreserveFormat,
            // Parâmetros da detecção, para que drones com frequência de pás
            // incomum sejam ajustados sem recompilar.
            "auto_band":                sync.audioAutoBand,
            "band_lo_hz":               sync.audioBandLo,
            "band_hi_hz":               sync.audioBandHi,
            "highpass_hz":              sync.audioHighpass
        };
    }
    property bool audioPreserveFormat: true;
    /// Se a banda das pás é detectada a partir do próprio sinal.
    property bool audioAutoBand: true;
    /// Banda fixa, usada quando audioAutoBand está desligado ou como fallback.
    property real audioBandLo: 150;
    property real audioBandHi: 900;
    /// Corte do passa-alta aplicado ao gyro, para remover movimento intencional.
    property real audioHighpass: 30;
    Timer {
        id: autosyncTimer;
        interval: 200;
        property bool doRun: false;
        running: controller.lens_loaded && controller.gyro_loaded && !window.isDialogOpened && doRun && render_queue.editing_job_id == 0;
        onTriggered: {
            doRun = false;
            if (controller.offsets_model.rowCount() == 0 && !window.motionData.hasAccurateTimestamps)
                autosync.doSync();
        }
    }
    function getSettings(): var {
        return {
            "initial_offset":     initialOffset.value,
            "initial_offset_inv": checkNegativeInitialOffset.checked,
            "search_size":        syncSearchSize.value,
            "calc_initial_fast":  calculateInitialOffsetFirst.checked,
            "max_sync_points":    maxSyncPoints.value,
            "every_nth_frame":    everyNthFrame.value,
            "time_per_syncpoint": timePerSyncpoint.value,
            "of_method":          syncMethod.currentIndex,
            "offset_method":      offsetMethod.currentIndex,
            "pose_method":        poseMethod.currentIndex,
            "auto_sync_points":   experimentalAutoSyncPoints.checked,
        };
    }
    function getSettingsJson(): string { return JSON.stringify(getSettings()); }

    // Pattern example, all values can be either frames, s or ms
    // {
    //     "start": "1001"    // frames
    //     "interval": "5s"   // s
    //     "gap": "100ms"     // ms
    // }
    // Keep in sync with render_queue.rs
    function resolveDurationToMs(d: var, fps: real): real {
        if (!d) return 0;
             if (d.toString().endsWith("ms")) return +(d.replace("ms", ""));
        else if (d.toString().endsWith("s"))  return +(d.replace("s", "")) * 1000.0;
        else                                  return (+d / fps) * 1000.0;
    }
    function resolveItem(x: var, duration: real, fps: real): list<var> {
        const start = x.hasOwnProperty("start")? resolveDurationToMs(x.start, fps) : 0;
        const interval = x.hasOwnProperty("interval")? resolveDurationToMs(x.interval, fps) : duration;
        const gap = resolveDurationToMs(x.gap, fps);
        let out = [];
        for (let i = start; i < duration; i += interval) {
            out.push(i - gap / 2.0);
            if (gap > 0) {
                out.push(i + gap / 2.0);
            }
        }
        return out;
    }
    function resolveSyncpointPattern(o: var): list<real> {
        const duration = window.videoArea.vid.duration;
        const fps      = window.videoArea.vid.frameRate;

        let timestamps = [];
        if (Array.isArray(o)) {
            for (const x of o) {
                timestamps.push(...resolveItem(x, duration, fps));
            }
        } else if (Object.isObject(o)) {
            timestamps.push(...resolveItem(o, duration, fps));
        }
        timestamps.sort((a, b) => a - b);

        return timestamps;
    }
    Connections {
        target: controller;
        function onTelemetry_loaded(is_main_video: bool, filename: string, camera: string, additional_data: var): void {
            sync.additionalSyncTimestamps = [];
            if (additional_data.additional_sync_points) {
                for (const x of additional_data.additional_sync_points.split(";")) {
                    sync.additionalSyncTimestamps.push(+x);
                }
            }
        }
    }

    Button {
        id: autosync;
        text: qsTr("Auto sync");
        iconName: "spinner"
        anchors.horizontalCenter: parent.horizontalCenter;
        // enabled: controller.gyro_loaded;
        tooltip: !enabled? qsTr("No motion data loaded, cannot sync.") : "";
        function doSync(): void {
            let maxPoints = maxSyncPoints.value;
            let sync_points = controller.get_optimal_sync_points(maxPoints, initialOffset.value);

            if (!sync_points || !experimentalAutoSyncPoints.checked) {
                let ranges = [];
                const trimRanges = videoArea.timeline.getTrimRanges();
                if (trimRanges.length > 1) {
                    maxPoints = Math.ceil(maxPoints / trimRanges.length) + 1;
                }
                for (const [trimStart, trimEnd] of trimRanges) {
                    const trimmed = trimEnd - trimStart;
                    const chunks = trimmed / maxPoints;
                    const start = trimStart + (chunks / 2);

                    for (let i = 0; i < maxPoints; ++i) {
                        const pos = start + (i*chunks);
                        ranges.push(pos);
                    }
                    const duration = window.videoArea.vid.duration;
                    const filter_ranges = v => (v >= trimStart * duration) && (v <= trimEnd * duration);
                    if (sync.customSyncTimestamps.length > 0) {
                        ranges = sync.customSyncTimestamps.filter(filter_ranges).map(v => v / duration);
                    }
                    if (sync.additionalSyncTimestamps.length > 0) {
                        for (const x of sync.additionalSyncTimestamps.filter(filter_ranges)) {
                            ranges.push(x / duration);
                        }
                    }
                }
                ranges.sort((a, b) => a - b);
                sync_points = ranges.join(";");
            }
            controller.start_autosync(sync_points, sync.getSettingsJson(), "synchronize");
        }
        onClicked: {
            if (!controller.lens_loaded) {
                messageBox(Modal.Warning, qsTr("Lens profile is not loaded, synchronization will most likely give wrong results. Are you sure you want to continue?"), [
                    { text: qsTr("Yes"), clicked: function() {
                        doSync();
                    }},
                    { text: qsTr("No"), accent: true },
                ]);
            } else {
                doSync();
            }
        }

        CheckBox {
            id: experimentalAutoSyncPoints;
            anchors.left: autosync.right;
            anchors.leftMargin: 5 * dpiScale;
            anchors.verticalCenter: parent.verticalCenter;
            contentItem.visible: false;
            scale: 0.7;
            tooltip: qsTr("Experimental automatic sync point selection.");
        }
    }

    InfoMessageSmall {
        property bool usesQuats: ((window.motionData.hasQuaternions && window.motionData.integrationMethod === 0) || window.motionData.hasAccurateTimestamps) && window.motionData.filename == window.vidInfo.filename;
        show: usesQuats && controller.offsets_model.rowCount() > 0;
        text: qsTr("This file uses synced motion data, additional sync points are not needed and can make the output look worse.");
        onUsesQuatsChanged: sync.opened = !usesQuats;
    }

    Label {
        position: Label.LeftPosition;
        text: qsTr("Rough gyro offset");

        NumberField {
            id: initialOffset;
            width: parent.width - checkNegativeInitialOffset.width;
            height: 25 * dpiScale;
            defaultValue: 0;
            precision: 1;
            unit: qsTr("s");
        }
        CheckBox {
            id: checkNegativeInitialOffset;
            anchors.left: initialOffset.right;
            anchors.leftMargin: 5 * dpiScale;
            anchors.verticalCenter: parent.verticalCenter;
            contentItem.visible: false;
            scale: 0.7;
            tooltip: qsTr("Analyze both positive and negative offset.\nThis doubles the calculation time, so check this only for the initial point and uncheck once you know the offset.");
        }
    }

    Label {
        position: Label.LeftPosition;
        text: qsTr("Sync search size");

        NumberField {
            id: syncSearchSize;
            width: parent.width - (calculateInitialOffsetFirst.visible? calculateInitialOffsetFirst.width : 0);
            height: 25 * dpiScale;
            precision: 1;
            value: 5;
            defaultValue: 5;
            unit: qsTr("s");
            onValueChanged: if (calculateInitialOffsetFirst.visible) calculateInitialOffsetFirst.checked = value > 10;
        }
        CheckBox {
            id: calculateInitialOffsetFirst;
            anchors.left: syncSearchSize.right;
            anchors.leftMargin: 5 * dpiScale;
            anchors.verticalCenter: parent.verticalCenter;
            contentItem.visible: false;
            scale: 0.7;
            visible: offsetMethod.currentIndex > 0;
            tooltip: qsTr("Calculate initial offset first (using essential matrix method), then refine using slower but more accurate rs-sync method.");
        }
    }
    Label {
        position: Label.LeftPosition;
        text: qsTr("Max sync points");

        NumberField {
            id: maxSyncPoints;
            width: parent.width;
            height: 25 * dpiScale;
            value: 3;
            from: 1;
            to: 30;
            onValueChanged: { if (value < 1) value = 1; if (value > 500) value = 500; }
        }
    }

    AdvancedSection {
        Label {
            position: Label.LeftPosition;
            text: qsTr("Analyze every n-th frame");

            NumberField {
                id: everyNthFrame;
                width: parent.width;
                height: 25 * dpiScale;
                value: 1;
                defaultValue: 1;
                from: 1;
            }
        }
        Label {
            position: Label.LeftPosition;
            text: qsTr("Time to analyze per sync point");

            NumberField {
                id: timePerSyncpoint;
                width: parent.width;
                height: 25 * dpiScale;
                value: 1.5;
                defaultValue: 1.5;
                precision: 2;
                unit: qsTr("s");
                from: 0.01;
            }
        }
        Label {
            position: Label.LeftPosition;
            text: qsTr("Processing resolution");
            ComboBox {
                id: processingResolution;
                model: [QT_TRANSLATE_NOOP("Popup", "Full"), "4k", "1080p", "720p", "480p"];
                font.pixelSize: 12 * dpiScale;
                width: parent.width;
                currentIndex: 3;
                onCurrentIndexChanged: {
                    let target_height = -1; // Full
                    switch (currentIndex) {
                        case 1: target_height = 2160; break;
                        case 2: target_height = 1080; break;
                        case 3: target_height = 720; break;
                        case 4: target_height = 480; break;
                    }

                    controller.set_processing_resolution(target_height);
                    render_queue.set_processing_resolution(target_height);
                }
            }
        }
        InfoMessageSmall {
            show: syncMethod.currentValue == "AKAZE";
            text: qsTr("The AKAZE method may be more accurate but is significantly slower than OpenCV. Use only if OpenCV doesn't produce good results");
        }
        Label {
            position: Label.LeftPosition;
            text: qsTr("Optical flow method");

            ComboBox {
                id: syncMethod;
                model: ["AKAZE", "OpenCV (PyrLK)", "OpenCV (DIS)"];
                font.pixelSize: 12 * dpiScale;
                width: parent.width;
                currentIndex: 2;
                onCurrentIndexChanged: controller.set_of_method(currentIndex);
                Component.onCompleted: currentIndexChanged();
            }
        }
        Label {
            text: qsTr("Pose method");
            position: Label.LeftPosition;

            ComboBox {
                id: poseMethod;
                model: ["findEssentialMat", "Almeida", "EightPoint", "findHomography"];
                font.pixelSize: 12 * dpiScale;
                width: parent.width;
                currentIndex: 0;
                onCurrentIndexChanged: controller.set_of_method(syncMethod.currentIndex);
            }
        }
        Label {
            text: qsTr("Offset method");
            position: Label.LeftPosition;

            ComboBox {
                id: offsetMethod;
                model: [QT_TRANSLATE_NOOP("Popup", "Essential matrix"), QT_TRANSLATE_NOOP("Popup", "Visual features"), QT_TRANSLATE_NOOP("Popup", "rs-sync")];
                font.pixelSize: 12 * dpiScale;
                width: parent.width;
                currentIndex: 2;
                property var tooltips: ([
                    qsTr("Calculate camera transformation matrix from optical flow to get the rotation angles of the camera.\nThen try to match these angles to gyroscope angles."),
                    qsTr("Undistort optical flow points using gyro and candidate offset.\nThen calculate lengths of the optical flow lines.\nResulting offset is the one where lines were the shortest, meaning the video was moving the least visually."),
                    qsTr("Rolling shutter video to gyro synchronization algorithm.\nMake sure you have proper rolling shutter value set before syncing.")
                ]);
                tooltip: tooltips[currentIndex];
            }
        }
        CheckBoxWithContent {
            id: lpfcb;
            text: qsTr("Low pass filter");
            onCheckedChanged: controller.set_sync_lpf(checked? lpf.value : 0);

            NumberField {
                id: lpf;
                unit: qsTr("Hz");
                precision: 2;
                value: 0;
                defaultValue: 0;
                from: 0;
                width: parent.width;
                onValueChanged: {
                    controller.set_sync_lpf(lpfcb.checked? lpf.value : 0);
                }
            }
        }
        CheckBox {
            id: showFeatures;
            text: qsTr("Show detected features");
            checked: true;
            onCheckedChanged: controller.show_detected_features = checked;
        }
        CheckBox {
            id: showOF;
            text: qsTr("Show optical flow");
            checked: true;
            onCheckedChanged: controller.show_optical_flow = checked;
        }

        // ---- Áudio externo ----
        // Importa uma trilha gravada em separado (DJI Mic e similares) para
        // alinhá-la ao vídeo. A waveform aparece como uma lane na timeline.
        Hr { }

        Label {
            position: Label.LeftPosition;
            text: qsTr("External audio");
            width: parent.width;

            Column {
                width: parent.width;
                spacing: 6 * dpiScale;

                FileDialog {
                    id: audioFileDialog;
                    property var extensions: ["wav", "m4a", "mp3", "flac", "aac", "mp4"];
                    title: qsTr("Choose an audio file");
                    nameFilters: [qsTr("Audio files") + " (*.wav *.m4a *.mp3 *.flac *.aac *.mp4)"];
                    type: "audio";
                    onAccepted: {
                        const waveform = window.videoArea.timeline.getAudioWaveform();
                        controller.import_external_audio(audioFileDialog.selectedFile, waveform);
                    }
                }

                Button {
                    text: audioInfo.text? qsTr("Replace audio file") : qsTr("Import external audio");
                    icon.name: "video";
                    width: parent.width;
                    onClicked: audioFileDialog.open2();
                }

                BasicText {
                    id: audioInfo;
                    width: parent.width;
                    wrapMode: Text.WordWrap;
                    leftPadding: 0;
                    // Mostra o formato detectado: é assim que o usuário confirma
                    // que o 32-bit float foi reconhecido como tal.
                    text: controller.get_external_audio_info();
                    visible: !!text;
                    Connections {
                        target: controller;
                        function onExternal_audio_changed() {
                            audioInfo.text = controller.get_external_audio_info();
                            audioPath.text = controller.get_external_audio_path();
                        }
                    }
                }
                BasicText {
                    id: audioPath;
                    width: parent.width;
                    wrapMode: Text.WrapAnywhere;
                    leftPadding: 0;
                    font.pixelSize: 11 * dpiScale;
                    opacity: 0.6;
                    text: controller.get_external_audio_path();
                    visible: !!text;
                }

                // Auto-sync: correlaciona a vibração das hélices captada no
                // áudio com a lida pelo giroscópio. Preenche o slider abaixo;
                // o usuário pode ajustar por cima.
                Button {
                    text: qsTr("Auto-sync audio");
                    icon.name: "sync";
                    width: parent.width;
                    visible: !!audioInfo.text;
                    enabled: controller.gyro_loaded;
                    onClicked: {
                        const raw = controller.auto_sync_external_audio(
                            sync.audioAutoBand, sync.audioBandLo, sync.audioBandHi, sync.audioHighpass);
                        if (!raw) {
                            autoSyncResult.text = qsTr("Not enough data to sync");
                            autoSyncResult.isWeak = true;
                            return;
                        }
                        const r = JSON.parse(raw);
                        audioOffset.value = r.offset_seconds * 1000.0;

                        // Confiança baixa quase sempre significa que a vibração
                        // das pás não chegou ao gyro (gimbal isolando) ou que o
                        // mic estava longe do drone.
                        autoSyncResult.isWeak = r.confidence < 0.3;
                        autoSyncResult.text = qsTr("Confidence: %1%").arg((r.confidence * 100).toFixed(0))
                                            + (autoSyncResult.isWeak? " — " + qsTr("weak match, check manually") : "");
                    }
                }
                BasicText {
                    id: autoSyncResult;
                    property bool isWeak: false;
                    width: parent.width;
                    leftPadding: 0;
                    wrapMode: Text.WordWrap;
                    font.pixelSize: 11 * dpiScale;
                    color: isWeak? "#cc8866" : styleTextColor;
                    visible: !!text && !!audioInfo.text;
                }

                // Offset manual. Arrastar apenas re-mapeia a posição de desenho
                // da waveform: nada do áudio é redecodificado.
                Label {
                    position: Label.LeftPosition;
                    text: qsTr("Offset");
                    visible: !!audioInfo.text;
                    width: parent.width;

                    SliderWithField {
                        id: audioOffset;
                        width: parent.width;
                        from: -30000;
                        to: 30000;
                        value: 0;
                        defaultValue: 0;
                        unit: qsTr("ms");
                        precision: 1;
                        live: true;
                        onValueChanged: {
                            const seconds = value / 1000.0;
                            const waveform = window.videoArea.timeline.getAudioWaveform();
                            if (waveform) waveform.offsetSeconds = seconds;
                            // A lane redesenha sozinha; o controller guarda o
                            // valor porque é dele que o export vai ler.
                            controller.set_external_audio_offset(seconds);
                        }
                    }
                }

                // O mesmo offset em frames, porque é assim que o usuário pensa
                // ao comparar com a timeline de um editor.
                BasicText {
                    width: parent.width;
                    leftPadding: 0;
                    visible: !!audioInfo.text;
                    opacity: 0.7;
                    text: {
                        const fps = controller.get_scaled_fps();
                        if (!fps) return "";
                        const frames = (audioOffset.value / 1000.0) * fps;
                        return qsTr("%1 frames @ %2 fps").arg(frames.toFixed(2)).arg(fps.toFixed(3));
                    }
                }

                // Selo de preservação de formato. Qualquer perda de precisão
                // precisa ser visível ANTES do export, nunca uma surpresa no
                // arquivo final.
                Rectangle {
                    id: formatBadge;
                    width: parent.width;
                    height: badgeText.height + 12 * dpiScale;
                    visible: !!audioInfo.text && !!badgeText.text;
                    radius: 4 * dpiScale;
                    color: formatBadge.isMismatch? "#40cc6666" : "#4066cc66";

                    property bool isMismatch: false;

                    function refresh(): void {
                        // A extensão manda: é o container do vídeo que decide se
                        // o áudio cabe. Vem do nome de arquivo escolhido no
                        // painel de exportação.
                        const filename = window.outputFile? window.outputFile.filename : "";
                        const dot = filename.lastIndexOf(".");
                        const ext = dot >= 0? filename.substring(dot + 1) : "";
                        if (!ext) { badgeText.text = ""; return; }

                        const raw = controller.get_external_audio_format_status(ext);
                        if (!raw) { badgeText.text = ""; return; }

                        const s = JSON.parse(raw);
                        formatBadge.isMismatch = s.status === "mismatch";
                        switch (s.status) {
                            case "preserved":
                                badgeText.text = qsTr("Audio: %1 preserved (%2)").arg(s.source_format).arg(s.codec);
                                break;
                            case "mismatch":
                                badgeText.text = qsTr("Audio: %1 does not fit in .%2. Switch the output to .%3 or the audio will be converted.")
                                                 .arg(s.source_format).arg(s.extension).arg(s.suggested_extension);
                                break;
                            case "downgrade":
                                badgeText.text = qsTr("Audio: will be converted to %1").arg(s.codec);
                                break;
                        }
                    }

                    BasicText {
                        id: badgeText;
                        anchors.centerIn: parent;
                        width: parent.width - 12 * dpiScale;
                        wrapMode: Text.WordWrap;
                        font.pixelSize: 11 * dpiScale;
                    }

                    Connections {
                        target: controller;
                        function onExternal_audio_changed() { formatBadge.refresh(); }
                    }
                    // O container pode mudar depois do áudio ser importado
                    // (trocar o codec de vídeo troca a extensão), e o selo tem
                    // que acompanhar.
                    Connections {
                        target: window.outputFile;
                        ignoreUnknownSignals: true;
                        function onFilenameChanged() { formatBadge.refresh(); }
                    }
                }

                CheckBox {
                    text: qsTr("Preserve original audio format");
                    checked: sync.audioPreserveFormat;
                    visible: !!audioInfo.text;
                    // Desligar é uma escolha explícita: com isto ligado (default)
                    // o áudio nunca é convertido sem aviso.
                    onCheckedChanged: {
                        sync.audioPreserveFormat = checked;
                        controller.set_external_audio_preserve_format(checked);
                        formatBadge.refresh();
                    }
                }

                Button {
                    text: qsTr("Remove audio");
                    width: parent.width;
                    visible: !!audioInfo.text;
                    onClicked: {
                        controller.clear_external_audio(window.videoArea.timeline.getAudioWaveform());
                        audioOffset.value = 0;
                    }
                }
            }
        }
    }
}
