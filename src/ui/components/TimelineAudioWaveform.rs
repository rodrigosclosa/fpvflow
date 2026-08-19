// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

#![allow(non_snake_case)]

//! Lane da timeline que desenha a forma de onda do áudio externo.
//!
//! Segue o mesmo padrão do [`super::TimelineGyroChart`]: um `QQuickPaintedItem`
//! que recebe `visibleAreaLeft`/`visibleAreaRight` do QML e converte tempo em
//! pixel com a mesma fórmula, para que a waveform fique alinhada ao gráfico do
//! gyro logo acima dela.
//!
//! O Rust reduz o áudio a pares (mínimo, máximo) por coluna de pixel — ver
//! [`gyroflow_core::audio::waveform`] — de modo que o volume de dados que chega
//! ao desenho independe da duração da trilha.

use gyroflow_core::audio::AudioTrack;
use qmetaobject::*;

use crate::util;

#[derive(Default, QObject)]
pub struct TimelineAudioWaveform {
    base: qt_base_class!(trait QQuickPaintedItem),

    /// Início da área visível da timeline, normalizado em `0.0..=1.0`.
    visibleAreaLeft: qt_property!(f64; WRITE setVisibleAreaLeft),
    /// Fim da área visível da timeline, normalizado em `0.0..=1.0`.
    visibleAreaRight: qt_property!(f64; WRITE setVisibleAreaRight),
    /// Escala vertical aplicada à amplitude.
    vscale: qt_property!(f64; WRITE setVScale),
    /// `"light"` ou `"dark"`, para escolher a cor do traço.
    theme: qt_property!(String; WRITE setTheme),

    /// Duração do **vídeo** em milissegundos.
    ///
    /// É o eixo temporal da timeline: a waveform é posicionada contra ele, e não
    /// contra a própria duração do áudio.
    durationMs: qt_property!(f64; WRITE setDurationMs),

    /// Deslocamento do áudio em segundos (`t_audio = t_video + offset`).
    ///
    /// Na Fase 1 permanece `0.0`. Na Fase 2 o slider passa a escrevê-lo, e o
    /// desenho se move sem que o áudio seja redecodificado.
    offsetSeconds: qt_property!(f64; WRITE setOffsetSeconds),

    /// Se há uma trilha carregada — o QML usa para mostrar ou ocultar a lane.
    hasAudio: qt_property!(bool; READ hasAudio NOTIFY audioChanged),
    audioChanged: qt_signal!(),

    /// Trilha carregada. `None` enquanto o usuário não importar nada.
    track: Option<AudioTrack>,

    /// Picos já calculados para a área visível: um par (mínimo, máximo) por
    /// coluna de pixel.
    peaks: Vec<(f32, f32)>,

    /// Duração da trilha em segundos, memorizada para não recalcular a cada
    /// repintura.
    audio_duration: f64,
}

impl TimelineAudioWaveform {
    fn setVisibleAreaLeft(&mut self, v: f64) { self.visibleAreaLeft = v; self.update(); }
    fn setVisibleAreaRight(&mut self, v: f64) { self.visibleAreaRight = v; self.update(); }
    fn setVScale(&mut self, v: f64) { self.vscale = v; self.update(); }
    fn setTheme(&mut self, v: String) { self.theme = v; self.update(); }
    fn setDurationMs(&mut self, v: f64) { self.durationMs = v; self.update(); }
    fn setOffsetSeconds(&mut self, v: f64) { self.offsetSeconds = v; self.update(); }

    fn hasAudio(&self) -> bool { self.track.is_some() }

    /// Instala a trilha decodificada e redesenha.
    pub fn set_track(&mut self, track: Option<AudioTrack>) {
        self.audio_duration = track.as_ref().map_or(0.0, |t| t.duration_seconds());
        self.track = track;
        self.audioChanged();
        self.update();
    }

    /// Recalcula os picos e agenda a repintura.
    ///
    /// O `qt_queued_callback` é o mesmo mecanismo usado pelo gráfico do gyro:
    /// garante que o `update` do item aconteça na thread da UI.
    pub fn update(&mut self) {
        self.calculate_peaks();
        util::qt_queued_callback(QPointer::from(self as &Self), |this, _| {
            (this as &dyn QQuickItem).update();
        })(());
    }

    /// Reduz o trecho visível do áudio a um par (mínimo, máximo) por pixel.
    ///
    /// Só o intervalo visível é processado: com zoom fechado num clipe longo,
    /// isso evita percorrer o arquivo inteiro a cada quadro.
    fn calculate_peaks(&mut self) {
        self.peaks.clear();

        let rect = (self as &dyn QQuickItem).bounding_rect();
        if rect.width <= 0.0 || rect.height <= 0.0 || self.durationMs <= 0.0 {
            return;
        }

        let Some(track) = &self.track else { return };
        if track.is_empty() {
            return;
        }

        let channels = track.channels as usize;
        let sample_rate = track.sample_rate as f64;
        if channels == 0 || sample_rate <= 0.0 {
            return;
        }

        let width = rect.width as usize;
        let duration_s = self.durationMs / 1000.0;

        // Instantes de vídeo nas bordas esquerda e direita da área visível.
        let visible_from_s = self.visibleAreaLeft * duration_s;
        let visible_to_s = self.visibleAreaRight * duration_s;
        if visible_to_s <= visible_from_s {
            return;
        }

        let seconds_per_pixel = (visible_to_s - visible_from_s) / rect.width;
        let total_frames = track.frame_count();

        self.peaks.reserve(width);
        for px in 0..width {
            // Converte a coluna de pixel para tempo de vídeo e daí para tempo de
            // áudio, aplicando o offset. `t_audio = t_video + offset`.
            let t_video = visible_from_s + px as f64 * seconds_per_pixel;
            let t_audio_start = t_video + self.offsetSeconds;
            let t_audio_end = t_audio_start + seconds_per_pixel;

            let first = (t_audio_start * sample_rate).floor();
            let last = (t_audio_end * sample_rate).ceil();

            // Fora dos limites da trilha o desenho fica plano — é assim que o
            // usuário enxerga onde o áudio começa e termina em relação ao vídeo.
            if last <= 0.0 || first >= total_frames as f64 {
                self.peaks.push((0.0, 0.0));
                continue;
            }

            let first_frame = (first.max(0.0) as usize).min(total_frames);
            let last_frame = (last.max(0.0) as usize).min(total_frames).max(first_frame + 1);

            let mut min = f32::MAX;
            let mut max = f32::MIN;
            let from = first_frame * channels;
            let to = (last_frame * channels).min(track.samples.len());
            for value in &track.samples[from..to] {
                if *value < min { min = *value; }
                if *value > max { max = *value; }
            }

            if min > max {
                self.peaks.push((0.0, 0.0));
            } else {
                self.peaks.push((min, max));
            }
        }
    }
}

impl QQuickItem for TimelineAudioWaveform {
    fn geometry_changed(&mut self, _new: QRectF, _old: QRectF) {
        self.update();
    }
}

impl QQuickPaintedItem for TimelineAudioWaveform {
    fn paint(&mut self, p: &mut QPainter) {
        if self.peaks.is_empty() {
            return;
        }

        let rect = (self as &dyn QQuickItem).bounding_rect();
        let half_height = rect.height / 2.0;

        // Verde-azulado, na mesma família das cores já usadas na timeline, mas
        // distinto das séries do gyro para não confundir as lanes.
        let color = if self.theme == "light" { "#3a7d7d" } else { "#63c5c5" };

        p.set_render_hint(QPainterRenderHint::Antialiasing, false);
        let mut pen = QPen::from_color(QColor::from_name(color));
        pen.set_width_f(1.0);
        p.set_pen(pen);
        p.set_brush(QBrush::default());

        let vscale = if self.vscale > 0.0 { self.vscale } else { 1.0 };

        // Uma linha vertical por coluna de pixel, do mínimo ao máximo daquele
        // trecho: é o desenho clássico de waveform e custa uma linha por pixel.
        let lines: Vec<QLineF> = self
            .peaks
            .iter()
            .enumerate()
            .map(|(px, (min, max))| {
                let x = px as f64;
                QLineF {
                    pt1: QPointF { x, y: half_height * (1.0 - (*max as f64 * vscale).clamp(-1.0, 1.0)) },
                    pt2: QPointF { x, y: half_height * (1.0 - (*min as f64 * vscale).clamp(-1.0, 1.0)) },
                }
            })
            .collect();

        p.draw_lines(lines.as_slice());
    }
}
