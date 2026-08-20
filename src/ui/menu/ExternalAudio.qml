// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

import QtQuick

import "../components/"

/**
 * Painel de áudio externo.
 *
 * Importa uma trilha gravada em separado (DJI Mic e similares), permite
 * alinhá-la ao vídeo — manualmente ou por correlação com a vibração das
 * hélices — e a embute no arquivo exportado preservando o formato de origem.
 *
 * A forma de onda aparece como uma lane na timeline, abaixo do gráfico do gyro.
 */
MenuItem {
    id: root;
    text: qsTr("External audio");
    iconName: "sound";
    objectName: "externalaudio";
    innerItem.enabled: window.videoArea.vid.loaded;

    /// Se o formato de origem deve ser preservado sem perda na exportação.
    property bool audioPreserveFormat: true;
    /// Se a banda das pás é detectada a partir do próprio sinal.
    property bool audioAutoBand: true;
    /// Banda fixa, usada quando audioAutoBand está desligado ou como fallback.
    property real audioBandLo: 150;
    property real audioBandHi: 900;
    /// Corte do passa-alta aplicado ao gyro, para remover movimento intencional.
    property real audioHighpass: 30;

    /// Se há uma trilha carregada. Controla a visibilidade de quase tudo aqui.
    readonly property bool hasAudio: !!audioInfo.text;

    function getAudioWaveform(): var {
        return window.videoArea && window.videoArea.timeline
             ? window.videoArea.timeline.getAudioWaveform()
             : null;
    }

    /// Estado da trilha gravado no `.gyroflow`.
    function getSettings(): var {
        const path = controller.get_external_audio_url();
        if (!path) return { };
        return {
            "path":                     path,
            "sample_rate":              controller.get_external_audio_sample_rate(),
            "offset_seconds":           audioOffset.value / 1000.0,
            "preserve_original_format": root.audioPreserveFormat,
            // Parâmetros da detecção, para que drones com frequência de pás
            // incomum sejam ajustados sem recompilar.
            "auto_band":                root.audioAutoBand,
            "band_lo_hz":               root.audioBandLo,
            "band_hi_hz":               root.audioBandHi,
            "highpass_hz":              root.audioHighpass
        };
    }

    function loadGyroflow(obj: var): void {
        // Chave própria no projeto. Arquivos salvos antes desta feature
        // simplesmente não a têm, e continuam abrindo normalmente.
        const a = obj.audio_sync || { };
        if (!a || !a.hasOwnProperty("path") || !a.path) return;

        if (a.hasOwnProperty("preserve_original_format")) root.audioPreserveFormat = !!a.preserve_original_format;
        if (a.hasOwnProperty("auto_band"))   root.audioAutoBand = !!a.auto_band;
        if (a.hasOwnProperty("band_lo_hz"))  root.audioBandLo   = +a.band_lo_hz;
        if (a.hasOwnProperty("band_hi_hz"))  root.audioBandHi   = +a.band_hi_hz;
        if (a.hasOwnProperty("highpass_hz")) root.audioHighpass = +a.highpass_hz;

        // O offset é aplicado depois do decode: recarregar o arquivo redetecta
        // canais, bit depth e float da origem, e só então faz sentido
        // reposicionar a waveform.
        const pendingOffset = a.hasOwnProperty("offset_seconds") ? +a.offset_seconds : 0.0;
        if (controller.import_external_audio_url(a.path, root.getAudioWaveform())) {
            audioOffset.value = pendingOffset * 1000.0;
        }
    }

    FileDialog {
        id: audioFileDialog;
        property var extensions: ["wav", "m4a", "mp3", "flac", "aac", "mp4"];
        title: qsTr("Choose an audio file");
        nameFilters: [qsTr("Audio files") + " (*.wav *.m4a *.mp3 *.flac *.aac *.mp4)"];
        type: "audio";
        onAccepted: controller.import_external_audio(audioFileDialog.selectedFile, root.getAudioWaveform());
    }

    Button {
        text: root.hasAudio? qsTr("Replace audio file") : qsTr("Import external audio");
        iconName: "sound";
        width: parent.width;
        onClicked: audioFileDialog.open2();
    }

    // Formato detectado. É aqui que o usuário confirma que o 32-bit float foi
    // reconhecido como tal.
    BasicText {
        id: audioInfo;
        width: parent.width;
        wrapMode: Text.WordWrap;
        leftPadding: 0;
        text: controller.get_external_audio_info();
        visible: !!text;

        Connections {
            target: controller;
            function onExternal_audio_changed() {
                audioInfo.text = controller.get_external_audio_info();
                audioPath.text = controller.get_external_audio_path();
                formatBadge.refresh();
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

    // ---- Auto-sync ----
    Button {
        text: qsTr("Auto-sync audio");
        iconName: "sync";
        width: parent.width;
        visible: root.hasAudio;
        enabled: controller.gyro_loaded;
        tooltip: qsTr("Aligns the audio by correlating propeller vibration picked up by the microphone with the vibration read by the gyroscope.");
        onClicked: {
            const raw = controller.auto_sync_external_audio(root.audioAutoBand, root.audioBandLo, root.audioBandHi, root.audioHighpass);
            if (!raw) {
                autoSyncResult.text = qsTr("Not enough data to sync");
                autoSyncResult.isWeak = true;
                return;
            }
            const r = JSON.parse(raw);
            audioOffset.value = r.offset_seconds * 1000.0;

            // Confiança baixa quase sempre significa que a vibração das pás não
            // chegou ao gyro (gimbal isolando) ou que o mic estava longe do drone.
            autoSyncResult.isWeak = r.confidence < 0.3;

            // Com câmeras que só entregam quaternions (DJI O4P e afins) não há
            // vibração de hélice no sinal do gyro, e o alinhamento é feito pelo
            // início do movimento — menos preciso, então vale dizer qual foi.
            const method = r.method === "onset"
                         ? qsTr("aligned by start of movement")
                         : qsTr("aligned by propeller vibration");

            autoSyncResult.text = qsTr("Confidence: %1%").arg((r.confidence * 100).toFixed(0))
                                + " — " + method
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
        visible: !!text && root.hasAudio;
    }

    // ---- Offset manual ----
    // Arrastar apenas re-mapeia a posição de desenho da waveform: nada do áudio
    // é redecodificado.
    Label {
        position: Label.LeftPosition;
        text: qsTr("Offset");
        visible: root.hasAudio;

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
                const waveform = root.getAudioWaveform();
                if (waveform) waveform.offsetSeconds = seconds;
                // A lane redesenha sozinha; o controller guarda o valor porque é
                // dele que a exportação vai ler.
                controller.set_external_audio_offset(seconds);
            }
        }
    }

    // O mesmo offset em frames, porque é assim que o valor é comparado com a
    // timeline de um editor.
    BasicText {
        width: parent.width;
        leftPadding: 0;
        visible: root.hasAudio;
        opacity: 0.7;
        font.pixelSize: 11 * dpiScale;
        text: {
            const fps = controller.get_scaled_fps();
            if (!fps) return "";
            const frames = (audioOffset.value / 1000.0) * fps;
            return qsTr("%1 frames @ %2 fps").arg(frames.toFixed(2)).arg(fps.toFixed(3));
        }
    }

    // ---- Preservação de formato ----
    // Qualquer perda de precisão precisa ser visível ANTES do export, nunca uma
    // surpresa no arquivo final.
    Rectangle {
        id: formatBadge;
        width: parent.width;
        height: badgeText.height + 12 * dpiScale;
        visible: root.hasAudio && !!badgeText.text;
        radius: 4 * dpiScale;
        color: formatBadge.isMismatch? "#40cc6666" : "#4066cc66";

        property bool isMismatch: false;

        function refresh(): void {
            // A extensão manda: é o container do vídeo que decide se o áudio
            // cabe. Vem do nome de arquivo escolhido no painel de exportação.
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

        // O container pode mudar depois do áudio ser importado (trocar o codec
        // de vídeo troca a extensão), e o selo tem que acompanhar.
        Connections {
            target: window.outputFile;
            ignoreUnknownSignals: true;
            function onFilenameChanged() { formatBadge.refresh(); }
        }
    }

    CheckBox {
        text: qsTr("Preserve original audio format");
        checked: root.audioPreserveFormat;
        visible: root.hasAudio;
        tooltip: qsTr("Keeps the original bit depth and sample rate. 32-bit float stays 32-bit float, with no silent conversion.");
        // Desligar é uma escolha explícita: com isto ligado (default) o áudio
        // nunca é convertido sem aviso.
        onCheckedChanged: {
            root.audioPreserveFormat = checked;
            controller.set_external_audio_preserve_format(checked);
            formatBadge.refresh();
        }
    }

    // ---- Parâmetros da detecção ----
    AdvancedSection {
        visible: root.hasAudio;

        CheckBoxWithContent {
            id: autoBandCb;
            text: qsTr("Detect propeller band automatically");
            checked: root.audioAutoBand;
            onCheckedChanged: root.audioAutoBand = checked;

            Label {
                position: Label.LeftPosition;
                text: qsTr("Band");
                visible: !autoBandCb.checked;

                Row {
                    spacing: 5 * dpiScale;
                    NumberField {
                        value: root.audioBandLo;
                        unit: qsTr("Hz");
                        width: 60 * dpiScale;
                        onValueChanged: root.audioBandLo = value;
                    }
                    BasicText { text: "—"; anchors.verticalCenter: parent.verticalCenter; }
                    NumberField {
                        value: root.audioBandHi;
                        unit: qsTr("Hz");
                        width: 60 * dpiScale;
                        onValueChanged: root.audioBandHi = value;
                    }
                }
            }
        }

        // Remove o movimento intencional do sinal do gyro, deixando só a vibração.
        Label {
            position: Label.LeftPosition;
            text: qsTr("Gyro high-pass");

            NumberField {
                value: root.audioHighpass;
                unit: qsTr("Hz");
                width: 60 * dpiScale;
                onValueChanged: root.audioHighpass = value;
            }
        }
    }

    Button {
        text: qsTr("Remove audio");
        width: parent.width;
        visible: root.hasAudio;
        onClicked: {
            controller.clear_external_audio(root.getAudioWaveform());
            audioOffset.value = 0;
            autoSyncResult.text = "";
        }
    }
}
