# Fase 0 — Relatório de reconhecimento

**Feature:** sincronização de áudio externo (DJI Mic / WAV 32-bit float) ao gyro do Gyroflow.
**Repo:** fork do Gyroflow v1.6.3, upstream `b5e8828f` ("Update telemetry-parser").
**Data:** 2026-08-18.

> Todas as afirmações abaixo foram verificadas lendo o código real. Cada uma cita `arquivo:linha`.
> Onde eu **não** consegui verificar (crates externos não baixados), está marcado explicitamente como **NÃO VERIFICADO**.

---

## ✅ Bloqueio da Fase 0 resolvido em 2026-08-19

**A toolchain foi instalada e o build limpo passou:** `just build` gerou `target/release/gyroflow.exe` (41 MB) em 28m38s, exit code 0, sem nenhum erro de compilação. As afirmações deste relatório agora rodam sobre um projeto que compila, e as fases seguintes podem exigir verde de `cargo build` / `clippy`.

Versões efetivamente instaladas: Rust 1.97.1, just 1.58.0, Qt 6.7.3 (msvc2019_64), ffmpeg 8.1, LLVM 19.1.7, OpenCV 4.12.0, Windows 11 SDK 22621. A receita e as armadilhas do ambiente estão na memória do projeto (`gyroflow-build-env-windows`).

Três pontos de atenção para builds futuros: usar `CARGO_BUILD_JOBS=2` (com 8 jobs o `rustc` é morto por falta de RAM, **sem emitir diagnóstico**); `_scripts/windows.just:19` foi corrigido para `cd "{{ExtDir}}"` (o caminho do projeto tem hífen) e **ainda não foi commitado**; e `ext/vcpkg/ports/opencv4/portfile.cmake` recebeu um patch de `PKG_CONFIG_EXECUTABLE` que **se perde se o vcpkg for re-clonado**.

<details>
<summary>Registro histórico do bloqueio (obsoleto)</summary>

### ⚠️ Bloqueio da Fase 0: build limpo não foi executado

O item 1 da Fase 0 pedia um build limpo sem alterações. **Não foi possível executar.** Esta máquina não tem nenhuma peça da toolchain:

| Ferramenta | Status |
|---|---|
| `rustc` / `cargo` | ausente (não há `~/.cargo`) |
| `just` | ausente |
| Qt 6.7.3, ffmpeg 8.1, OpenCV, LLVM | ausentes — a pasta `ext/` não existe |

Portanto **nenhuma afirmação deste relatório foi validada por compilação**, e as fases seguintes não poderão declarar `cargo build` / `clippy` verdes até que a toolchain exista.

### Passos exatos para compilar neste ambiente

Fonte: `README.md:240-248` e `_scripts/windows.just` / `_scripts/common.just`.

```powershell
# 0. Pré-requisitos: git, 7z, PowerShell com scripts habilitados
set-executionpolicy remotesigned        # como admin, uma vez

# 1. Rust estável via https://rustup.rs/
#    Ao instalar o C++ build tools do Visual Studio Installer,
#    marcar o pacote de idioma inglês (README.md:242)

# 2. just
cargo install --force just

# 3. Dependências (baixa ~vários GB e COMPILA OpenCV via vcpkg — leva ~1h)
cd "C:\Users\Rodrigo Sclosa\Documents\Projetos\gyroflow\gyroflow"
just install-deps

# 4. Build/run
just run          # cargo run --release
just build        # cargo build --release
just clippy       # cargo clippy
```

Versões fixadas em `_scripts/common.just`: Qt **6.7.3** msvc2019_64 (linha 12), ffmpeg **`ffmpeg-8.1-windows-desktop-vs2026-gpl-lite`** (linha 21), LLVM **19.1.7** (`windows.just`), OpenCV 4 via vcpkg. Tudo é instalado em `ext/` dentro do repo, e `common.just:70` injeta esses caminhos no `PATH`.

</details>

**Dica de desenvolvimento:** `src/gyroflow.rs:51` tem `let ui_live_reload = false;` — mudando para `true`, alterações em QML recarregam sem recompilar (`README.md:238`). Isso acelera muito as Fases 1, 2 e 5 — especialmente agora, que sabemos que um build completo leva ~29 min nesta máquina.

---

## 1. Estrutura geral

Dois crates:

- **`gyroflow-core`** (`src/core/`, lib `gyroflow_core`) — lógica pura de estabilização. **Não depende de ffmpeg.**
- **`gyroflow`** (raiz, bin `src/gyroflow.rs`) — UI Qt + rendering. **É aqui que vive o ffmpeg.**

Módulos do core, em `src/core/lib.rs:5-27`:
```
gyro_source, imu_integration, lens_profile, lens_profile_database, calibration,
synchronization, stabilization, camera_identifier, keyframes, stmap, zooming,
smoothing, filtering, filesystem, gyro_export, settings, gpu, util, stabilization_params
```
→ um `pub mod audio;` novo entra nessa lista.

### Consequência de projeto: onde o módulo `audio/` deve morar

O prompt pede tudo em `src/core/audio/`. Há um conflito real:

- `decode.rs`, `waveform.rs`, `features.rs`, `sync.rs` são lógica pura → cabem no core sem problema.
- **`export.rs` precisa do ffmpeg**, e `gyroflow-core/Cargo.toml` **não tem ffmpeg** (verificado: a lista de dependências vai de `telemetry-parser` a `wgpu`, sem nenhuma entrada ffmpeg).

**Proposta:** manter `mod.rs`/`decode.rs`/`waveform.rs`/`features.rs`/`sync.rs` em `src/core/audio/` e colocar o encode/mux em `src/rendering/audio_export.rs`, junto do resto do ffmpeg. O `export.rs` do core, se existir, fica só com a lógica pura de montagem do buffer (offset, trim, silêncio) — que é testável sem ffmpeg. Isso preserva o espírito da regra ("isole a lógica nova") sem arrastar ffmpeg para dentro do core. **Preciso do seu OK nisso.**

---

## 2. GyroSource e os dados do giroscópio

**Struct:** `src/core/gyro_source/mod.rs:44`.

```rust
pub struct GyroSource {
    pub duration_ms: f64,
    raw_imu: Vec<TimeIMU>,              // linha 49 — privado
    pub quaternions: TimeQuat,
    pub file_metadata: ReadOnlyFileMetadata,
    offsets: BTreeMap<i64, f64>,        // linha 69 — <ts_us, offset_ms>
    offsets_linear: BTreeMap<i64, f64>,
    offsets_adjusted: BTreeMap<i64, f64>,
    pub file_url: String,
    // ...
}
```

**Tipo das amostras:** `src/core/gyro_source/mod.rs:33`
```rust
pub type TimeIMU = telemetry_parser::util::IMUData;
```
O crate `telemetry-parser` é git (`Cargo.lock:4284-4286`, rev `77a3b81`) e **não está baixado**, então não li a definição. Inferi os campos pelo uso real:

- `x.timestamp_ms` — `f64` (`optimsync.rs:49-50`, aritmética com `f64`)
- `x.gyro` — `Option<[f64;3]>` (`optimsync.rs:47`: `Vector3::from_column_slice(&left.gyro.unwrap_or_default())`)
- `x.accl` — `Option<[f64;3]>` (`mod.rs:831`)

→ **`[f64;3]`, não `[f32;3]`**. **NÃO VERIFICADO** diretamente; confirmar após `just install-deps`.

**Acesso programático:** `src/core/gyro_source/mod.rs:689`
```rust
pub fn raw_imu<'a>(&'a self, file_metadata: &'a FileMetadata) -> &'a Vec<TimeIMU>
```
Note que exige passar o `FileMetadata` — o campo próprio `raw_imu` costuma estar vazio e o dado real vem de `file_metadata.raw_imu` (`file_metadata.rs:54`). A partir do manager:
```rust
let gyro = stab.gyro.read();                       // src/core/lib.rs:83
let md   = gyro.file_metadata.read();
let imu  = gyro.raw_imu(&md);
```
Locks são `parking_lot::RwLock` dentro de `Arc` (`lib.rs:83-109`).

**Taxa de amostragem:** **não existe campo `imu_rate`** (grep vazio em `src/core/gyro_source/*.rs`). Terá que ser derivada dos timestamps: `N / (t_last - t_first)`. Tipicamente 200–1000 Hz conforme a câmera. Isso importa para o Nyquist do gyro na Fase 5.

**Offsets já existentes:** sim — `set_offset(timestamp_us, offset_ms)` (`mod.rs:694`) e `set_offsets(BTreeMap)` (`mod.rs:717`). Esse é o offset **gyro↔vídeo** do sync óptico. **Nosso offset de áudio é uma grandeza diferente** (áudio↔vídeo) e deve ser um campo separado — não reaproveitar esse `BTreeMap`.

---

## 3. Render / export e o ffmpeg

### 3.1 O fork do ffmpeg

**Não existe fork.** `Cargo.toml:57` usa `ffmpeg-next = "8.1.0"` direto do crates.io, e a seção `[patch.crates-io]` (linhas 65-68) só redireciona `qmetaobject` e `qttypes`. **Nenhuma modificação de fork será necessária** — isso remove o risco de aumento de escopo que a Fase 3 antecipava.

Arquivos: `src/rendering/` — `ffmpeg_processor.rs` (orquestrador, 681 linhas), `ffmpeg_audio.rs` (127), `audio_resampler.rs` (137), `ffmpeg_video.rs`, `ffmpeg_hw.rs`, `mod.rs`, `render_queue.rs`.

### 3.2 (a) Dá para injetar um stream de áudio próprio?

**Sim.** A API já é usada hoje: `ffmpeg_audio.rs:25` faz `octx.add_stream(codec)?` e `ffmpeg_processor.rs:337/348` também.

**Mas há uma ressalva importante.** Os streams de saída são montados num laço sobre os streams **de entrada** (`ffmpeg_processor.rs:290`):

```rust
for (i, stream) in self.input_context.streams().enumerate() {
    let medium = stream.parameters().medium();
    ...
    } else if medium == media::Type::Audio && self.audio_codec != codec::Id::None {   // :333
        if self.preserve_other_tracks { /* stream copy */ }                            // :335-339
        else { atranscoders.insert(i, AudioTranscoder::new(...)); }                    // :343
        output_index += 1;
    }
}
```

Ou seja: **se o vídeo de entrada não tiver trilha de áudio, nenhum stream de áudio é criado.** Para o nosso caso (áudio externo, possivelmente sobre um clipe mudo) será preciso adicionar um stream **fora** desse laço, depois dele. Isso é uma mudança pequena e localizada, mas é uma mudança em código existente — e é obrigatória.

### 3.3 (b) O áudio é copiado ou reencodado?

**Reencodado por padrão.** `AudioTranscoder` (`ffmpeg_audio.rs:9-14`) tem `decoder`, `encoder` e um `AudioResampler`, e o fluxo decodifica → resample → encoda (`ffmpeg_audio.rs:85-88`). Stream copy só acontece quando `preserve_other_tracks` está ligado (`ffmpeg_processor.rs:335`).

Isso é **excelente notícia**: o pipeline já sabe criar encoder de áudio, resample e escrever packets. Nosso trabalho é substituir a fonte dos frames (arquivo externo em vez do decoder do vídeo), não construir o mux do zero.

### 3.4 (c)(d) Codecs e containers

Codecs oferecidos hoje — `src/rendering/mod.rs:250-256`:
```rust
"AAC"         => codec::Id::AAC,
"PCM (s16le)" => codec::Id::PCM_S16LE,
"PCM (s16be)" => codec::Id::PCM_S16BE,
"PCM (s24le)" => codec::Id::PCM_S24LE,
"PCM (s24be)" => codec::Id::PCM_S24BE,
```
Mesma lista na UI: `src/ui/menu/Export.qml:597`. Default AAC (`ffmpeg_processor.rs:213`).

**`pcm_f32le` NÃO existe hoje** (grep por `PCM_F32LE`/`pcm_f32` em `src/` retorna só formatos de *pixel* `GBRPF32LE`). Precisamos adicionar `codec::Id::PCM_F32LE` ao match e uma entrada `"PCM (f32le)"` no ComboBox. É trivial — `PCM_F32LE` é um `codec::Id` padrão do ffmpeg-next, não requer fork.

**Container:** segue a **extensão do arquivo de saída** — `ffmpeg_processor.rs:271`
```rust
let mut output_format = if let Some(pos) = output_filename.rfind('.') { &output_filename[pos+1..] } else { "mp4" }.to_ascii_lowercase();
if output_format == "mkv" { output_format = String::from("matroska"); }   // :276
let mut octx = format::output_as_with(&file.path, &output_format, output_options)?;  // :282
```
E a extensão é ditada pelo codec de vídeo escolhido — `Export.qml:26-32`: H.264/H.265 → `.mp4`; ProRes / DNxHD / CineForm → `.mov`; EXR/PNG → sequência, `"audio": false`.

**Conclusão para a Fase 3:** `pcm_f32le` cabe em **MOV** e **MKV**, não em MP4. Como MP4 é o container dos codecs mais usados (H.264/H.265), o conflito previsto na decisão 7 **vai acontecer na prática, com frequência**. A UI precisará avisar e oferecer a troca — não é um caso de borda raro. Vale notar que `.mkv` não aparece hoje na lista de extensões do `Export.qml`, então "trocar para MKV" pode exigir expor essa opção.

### 3.5 Trim no render

`StabilizationParams::trim_ranges: Vec<(f64, f64)>` — `src/core/stabilization_params.rs:98`. São **múltiplos ranges**, em fração normalizada 0..1 da duração.

Conversão para o processor — `src/rendering/mod.rs:279`:
```rust
proc.ranges_ms = trim_ranges.iter()
    .map(|x| (if x.0 > 0.0 { Some(x.0 * duration_ms) } else { None },
              if x.1 < 1.0 { Some(x.1 * duration_ms) } else { None }))
    .collect();
```
→ `ranges_ms: Vec<(Option<f64>, Option<f64>)>` (`ffmpeg_processor.rs:40`).

Mecanismo de execução (`ffmpeg_processor.rs:481-492`): para cada range faz `input_context.seek(...)` e acumula `frame_ts.add_audio` / `add_video`, de modo que os segmentos são concatenados num timeline contínuo de saída. O corte de áudio hoje é feito por comparação de timestamp por frame (`ffmpeg_audio.rs:77`, `:96`) — precisão de frame de áudio (~1024 samples), **não de sample**.

**Implicação para a Fase 4:** para atingir a precisão de sample que o prompt pede, teremos que fatiar nosso buffer `f32` em índices de sample calculados por nós (`round((t + offset) * sample_rate)`) e alimentar o encoder com frames já cortados — em vez de depender do descarte por timestamp. Isso é factível porque controlamos o buffer inteiro.

Detalhe: `render_options.pad_with_black` e `preserve_other_tracks` desativam o uso de `ranges_ms` (`mod.rs:278`), e ranges viram exports separados se `export_trims_separately` (`mod.rs:697`, `render_queue.rs:1071`).

---

## 4. A flag de exportar áudio

Cadeia completa, ponta a ponta:

| Camada | Local | Nome |
|---|---|---|
| UI (checkbox) | `src/ui/menu/Export.qml:505-506` | `CheckBox { id: audio; text: "Export audio" }` |
| alias | `Export.qml:82`, `:109` | `exportAudio`, `outAudio` |
| serialização | `Export.qml:131` | `audio: root.outAudio` |
| struct Rust | `src/rendering/render_queue.rs:79` | `pub audio: bool` |
| consumo | `src/rendering/mod.rs:416-418` | `if !render_options.audio { proc.audio_codec = codec::Id::None; }` |

O gate real é `proc.audio_codec == codec::Id::None`, testado em `ffmpeg_processor.rs:333`, `:415`, `:465`, `:509`. **É exatamente o interruptor mestre que o prompt descreve** — não precisamos de flag nova.

Dois casos desligam o áudio à revelia do usuário, e ambos afetam a feature:
- `mod.rs:446` — mudança de velocidade do vídeo desativa áudio ("Audio not supported when changing speed").
- `Export.qml:283-284` — formatos com `"audio": false` (EXR/PNG) desabilitam e desmarcam o checkbox.

---

## 5. Timeline QML e a ponte Rust↔QML

**Como o gyro é desenhado:** **não é Canvas QML.** É um `QQuickPaintedItem` implementado **em Rust** — `src/ui/components/TimelineGyroChart.rs:53-55`:
```rust
#[derive(Default, QObject)]
pub struct TimelineGyroChart {
    base: qt_base_class!(trait QQuickPaintedItem),
    visibleAreaLeft:  qt_property!(f64; WRITE setVisibleAreaLeft),
    visibleAreaRight: qt_property!(f64; WRITE setVisibleAreaRight),
    vscale: qt_property!(f64; WRITE setVScale),
    theme: qt_property!(String),
    ...
}
```

**Como os dados chegam:** **não passam pelo QML.** O QML só entrega o *ponteiro do objeto*, e o Rust preenche os dados direto — `src/controller.rs:576-596`:
```rust
fn update_chart(&mut self, chart: QJSValue, series: String) -> bool {
    if let Some(chart) = chart.to_qobject::<TimelineGyroChart>() {
        let chart = unsafe { &mut *chart.as_ptr() };
        ...
        chart.setSyncResults(&est_gyro);
    }
}
```
Mesmo padrão em `FrequencyGraph.rs:53` (`pub fn setData(&mut self, vec: &[f64], sr: f64)`) e desenho em `paint(&mut self, p: &mut QPainter)` (`FrequencyGraph.rs:145`).

→ **Este é o padrão a seguir para a waveform.** Nada de serializar milhões de samples para QVariantList; os peak buckets ficam em Rust e são desenhados no `paint()`. `FrequencyGraph.rs` é o molde mais próximo (já faz FFT com janela + pintura).

**Zoom:** `src/ui/components/Timeline.qml:21-22, 40-41`
```qml
property real visibleAreaLeft:  0.0;
property real visibleAreaRight: 1.0;
function mapToVisibleArea(pos)   { return (pos - visibleAreaLeft) / (visibleAreaRight - visibleAreaLeft); }
function mapFromVisibleArea(pos) { return pos * (visibleAreaRight - visibleAreaLeft) + visibleAreaLeft; }
```
Fração 0..1 da duração total, propagada para o chart em `Timeline.qml:303-304`.

**Onde encaixar a lane da waveform:** dentro do `Item { id: inner }` (`Timeline.qml:285`), no `Rectangle` que hoje contém o `TimelineGyroChart` (`Timeline.qml:301`, `anchors.fill: parent`). Para uma lane separada, dividir a altura desse retângulo entre gyro e waveform, ou adicionar um `Rectangle` irmão abaixo. Os botões de eixo à direita (`Timeline.qml:272-283`) mostram o padrão de controles laterais.

**Registro de tipos novos:**
1. declarar o módulo em `src/gyroflow.rs:23` (`pub mod components { pub mod TimelineGyroChart; ... }`);
2. registrar em `src/gyroflow.rs:92-94`:
   ```rust
   qml_register_type::<TimelineGyroChart>(cstr::cstr!("Gyroflow"), 1, 0, cstr::cstr!("TimelineGyroChart"));
   ```
Arquivos **QML** novos são compilados automaticamente (`build.rs:88` varre `src/ui/`), mas componentes QML precisam ser listados em `src/ui/components/qmldir`.

**Painel para o botão "Import external audio":** o candidato natural é um painel novo logo após `MotionData`, seguindo `src/ui/App.qml:178-181`:
```qml
ItemLoader { id: motionData; sourceComponent: Component { Menu.MotionData { } } }
```
O padrão de importação de arquivo está em `src/ui/menu/MotionData.qml:26-41` (`FileDialog` + `onAccepted: loadFile(selectedFile)` → `controller.load_telemetry(url, ...)`), e `loadGyroflow(obj)` (`MotionData.qml:43`) mostra como um painel restaura seu estado do projeto.

**Slider da Fase 2:** `src/ui/components/SliderWithField.qml` já tem `from`/`to`/`unit`/`precision`/`defaultValue` (linhas 11-25) — atende slider + campo numérico + unidade sem componente novo.

---

## 6. Arquivo de projeto `.gyroflow`

**Escrita:** `src/core/lib.rs:1254` `export_gyroflow_data(...)`, JSON montado em `:1284`:
```rust
let mut obj = serde_json::json!({
    "title": "Gyroflow data file",
    "version": 4,                    // :1286
    "app_version": ...,
    "videofile": input_file.url,
    "calibration_data": ...,
    "video_info": { "fps", "duration_ms", "num_frames", ... },   // :1301-1309
    "gyro_source": { ... },                                       // ~:1340-1354
    "offsets": gyro.get_offsets(),                                // :1356
    "keyframes": ...,
    "trim_ranges_ms": trim_ranges_ms,                             // :1360
});
```
→ o bloco `"audio_sync"` novo entra ao lado de `"trim_ranges_ms"` (`lib.rs:1360`).

**Leitura:** `src/core/lib.rs:1458` `import_gyroflow_data(...)`. Versão lida em `:1463`:
```rust
load_options.project_version = obj.get("version").and_then(|x| x.as_u64()).unwrap_or(2);
```

**Retrocompatibilidade:** o formato **não** usa structs serde — é `serde_json::Value` com acessos opcionais. Padrão real (`lib.rs:1787`):
```rust
if let Some(ranges) = obj.get("trim_ranges_ms").and_then(|x| x.as_array()) { ... }
else if let Some(ranges) = obj.get("trim_ranges").and_then(|x| x.as_array()) { ... }
```
→ **adicionar campos novos é naturalmente retrocompatível**: basta ler com `.get(...).and_then(...)` e ter default quando ausente. Não é preciso subir a `version` para 5.

---

## 7. Sincronização existente e FFT

**Módulo:** `src/core/synchronization/` — `mod.rs`, `autosync.rs`, `optimsync.rs`, `estimate_pose/`, `find_offset/`, `optical_flow/`.

API principal — `src/core/synchronization/mod.rs:382`:
```rust
pub fn find_offsets<F: Fn(f64) + Sync>(&self, ranges: &[(i64, i64)], sync_params: &SyncParams,
    params: &ComputeParams, progress_cb: F, cancel_flag: Arc<AtomicBool>)
    -> Vec<(f64, f64, f64)>   // (timestamp, offset, cost)
```
O terceiro valor (`cost`) é o análogo do nosso *score de confiança* — há precedente na base para reportar qualidade de casamento.

O resultado é aplicado via `gyro.set_offset(ts_us, offset_ms)` (`gyro_source/mod.rs:694`). **Recomendo NÃO reusar esse `BTreeMap`** para o áudio: ele modela offset gyro↔vídeo, e misturar as duas grandezas quebraria o sync óptico. Campo separado em `AudioTrack`.

**rustfft:** já é dependência dos **dois** crates (`Cargo.toml:47` na raiz, `src/core/Cargo.toml` linha `rustfft = "6.4.1"`) e já é usado em `src/core/synchronization/optimsync.rs:7,84` (`FftPlanner`) e `src/ui/components/FrequencyGraph.rs:6-8` (`Radix4`). **A Fase 5 não precisa de dependência nova.**

**rodio:** só toca um som de notificação — `src/controller.rs:2370-2372`. **Não** é infraestrutura de decode de áudio; não serve para a Fase 1.

**symphonia:** presente no `Cargo.lock:4159` (v0.5.5) mas apenas transitivamente via rodio, com features `vorbis` + `ogg` — **sem suporte a WAV/PCM**. Para a Fase 1 seria preciso adicioná-lo como dependência direta com as features certas, ou usar `hound`, ou reusar o ffmpeg que já está no crate raiz.

**Recomendação de decode (Fase 1):** usar o **ffmpeg** que já está linkado no crate `gyroflow`. Justificativa: (a) zero dependências novas; (b) `format::Sample::F32` do ffmpeg preserva float nativamente; (c) já sabemos resamplear/reencodar com ele (`audio_resampler.rs`), então o mesmo formato de origem atravessa decode→export sem tradução extra; (d) suporta muito mais que WAV (o usuário pode ter .m4a do DJI Mic). O custo é que o decode fica no crate raiz, não no core — mesmo trade-off do `export.rs` da seção 1.

---

## 8. Filesystem

Abstração própria (Android/iOS/sandbox) em `src/core/filesystem/mod.rs`:
```rust
pub fn open_file(url: &str, writing: bool, truncate: bool) -> Result<FileWrapper>  // :490
pub fn read(url: &str) -> Result<Vec<u8>>                                          // :398
pub fn get_filename(url: &str) -> String                                           // :198
pub fn path_to_url(path: &str) -> String                                           // :507
pub fn url_to_path(url: &str) -> String                                            // :521
pub fn start_accessing_url(url: &str, is_folder: bool)                             // :102
```
O áudio deve ser aberto por essa API (com `start_accessing_url` / `stop_accessing_url` em volta), não com `std::fs` direto, para não quebrar em plataformas sandboxed.

---

## 9. Resumo dos impactos em código existente

| Arquivo | Mudança necessária | Fase |
|---|---|---|
| `src/core/lib.rs` | `pub mod audio;`; campo `audio` no manager; serializar/ler `"audio_sync"` | 1, 2 |
| `src/gyroflow.rs:23,92` | declarar + registrar `TimelineAudioWaveform` | 1 |
| `src/ui/components/qmldir` | registrar componente QML novo | 1 |
| `src/ui/components/Timeline.qml:285-310` | lane da waveform | 1 |
| `src/ui/App.qml:178` | painel novo de áudio | 1 |
| `src/controller.rs` | `qt_method!`s: importar áudio, alimentar waveform, auto-sync | 1, 2, 5 |
| `src/rendering/ffmpeg_processor.rs:290-350` | criar stream de áudio quando o input não tem áudio | 3 |
| `src/rendering/mod.rs:250-256` | adicionar `"PCM (f32le)" => codec::Id::PCM_F32LE` | 3 |
| `src/ui/menu/Export.qml:597` | adicionar `"PCM (f32le)"` ao ComboBox + selo de preservação | 3 |
| `src/rendering/render_queue.rs:79-89` | campos de áudio externo no `RenderOptions` | 3 |

---

## 10. Riscos e pontos que precisam da sua decisão

1. **Build não validado.** Nada aqui foi compilado. Precisa instalar a toolchain antes da Fase 1, ou aceitar que as fases sejam entregues sem verde de compilação.

2. ~~**Onde fica o `export.rs`** (seção 1). O core não tem ffmpeg.~~
   **✅ DECIDIDO em 2026-08-19: lógica pura no core, encode/mux em `src/rendering/audio_export.rs`.**
   Não é preferência: `gyroflow-core` não tem ffmpeg entre suas dependências, e adicioná-lo quebraria a propriedade que define o crate — ser lógica pura, compilável e testável sem ffmpeg. A divisão fica:
   - `src/core/audio/{mod,decode,waveform,features,sync}.rs` — montagem de buffer, offset, trim, silêncio. Testável sem ffmpeg.
   - `src/rendering/audio_export.rs` — encode e mux, junto do resto do ffmpeg, seguindo o que o projeto já faz.

3. ~~**`pcm_f32le` × MP4 não é caso de borda.** H.264/H.265 → `.mp4` (`Export.qml:26-27`), e MP4 não aceita PCM float.~~
   **✅ DECIDIDO em 2026-08-19: sugerir `.mov`, sem expor MKV na UI.** Três evidências no código:
   - `.mov` já é o container de 3 dos 7 formatos de export (ProRes, DNxHD, CineForm — `Export.qml:28-30`). Nada novo precisa aparecer na UI.
   - `App.qml:729` já tem a mensagem oficial do projeto para incompatibilidade codec×container: *"Make sure your output extension supports the selected codec. \".mov\" should work in most cases."* O Gyroflow **já recomenda MOV** exatamente nessa situação — basta reusar.
   - MKV funciona no pipeline (`ffmpeg_processor.rs:277` mapeia `mkv` → `matroska`) mas **não é oferecido como saída**, só aceito na entrada. Continua disponível para quem digitar a extensão à mão.

4. ~~**Tipo do gyro é `f64`, não `f32`** (inferido). O prompt assumia `[f32;3]`. Confirmar quando o `telemetry-parser` for baixado.~~
   **✅ RESOLVIDO em 2026-08-19.** Verificado no fonte real (`telemetry-parser` rev `77a3b81`, `src/util.rs:289-294`):
   ```rust
   pub struct IMUData {
       pub timestamp_ms: f64,
       pub gyro: Option<[f64; 3]>,
       pub accl: Option<[f64; 3]>,
       pub magn: Option<[f64; 3]>
   }
   ```
   É `[f64;3]` mesmo. **Consequência para a Fase 1:** o gyro chega em `f64` e o WAV 32-bit float em `f32` — a conversão precisa ser explícita, e é preciso decidir em que precisão rodar a correlação áudio↔gyro.

5. **Não há taxa de amostragem do gyro pronta** — terá que ser estimada dos timestamps. Relevante para o Nyquist na Fase 5.
   **⬇️ RISCO REDUZIDO em 2026-08-19.** `optimsync.rs:34-51` já resolve isso, e melhor do que estimar taxa:
   - `:34-35` — `duration_ms` (último − primeiro timestamp) e contagem de amostras com `gyro.is_some()` dão a taxa média sem precisar assumi-la.
   - `:40-51` — **interpolação linear por timestamp**: `partition_point` acha o par de amostras que cerca o instante desejado e interpola entre elas.

   **Consequência para a Fase 1:** não é preciso estimar taxa do gyro nem reamostrá-lo. Basta definir a grade temporal pelo lado do áudio (taxa fixa e conhecida, 48 kHz no DJI Mic) e consultar o gyro nesses instantes reusando o mesmo padrão de interpolação. Há precedente no próprio código a seguir. A estimativa de taxa continua necessária só para o critério de Nyquist da Fase 5.

6. **Precisão de sample no trim exige contornar o mecanismo atual**, que descarta por timestamp de frame (`ffmpeg_audio.rs:77`). Como montaremos o buffer inteiro, é viável — mas significa que o caminho do áudio externo não reusa `AudioTranscoder` tal como está.

7. **Áudio é desativado quando a velocidade do vídeo muda** (`mod.rs:446`). O áudio externo herdará essa limitação, salvo decisão contrária.
