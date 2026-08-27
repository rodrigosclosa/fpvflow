# Fase 0 — Reconhecimento: LUT e painel de cor

Levantamento do código real antes de escrever qualquer linha da feature. Todas as
afirmações citam `arquivo:linha`.

> **Leia primeiro a seção "Bloqueador".** Ela muda o desenho da feature e precisa
> de decisão antes da Fase 1.

---

## Bloqueador: a GPU processa planos YUV, não pixels RGB

Este é o achado que mais afeta o plano original, e não estava previsto na
especificação.

O kernel de estabilização é invocado **uma vez por plano de imagem**, não uma vez
por pixel RGB:

```rust
// src/rendering/mod.rs:735-738
let mut undistort_frame = |frame: &mut Video, out_frame: &mut Video| {
    for (i, cb) in planes.iter_mut().enumerate() {
        (*cb)(timestamp_us, frame, out_frame, i, fill_with_background);
    }
```

Cada invocação recebe seu índice em `kernel_params.plane_index`
(`src/rendering/mod.rs:616`), e o shader usa isso para saber se está tratando luma
ou croma (`stabilize_spirv/src/drawing.rs:78`, `cpu_undistort.rs:533`).

Para material YUV 4:2:0 — o caso normal de câmera — isso significa:

- o plano **Y** roda sozinho, sem acesso a U e V;
- os planos **U/V** têm **metade da resolução** do Y;
- nenhum shader enxerga R, G e B do mesmo pixel ao mesmo tempo.

**Um LUT 3D exige RGB.** Ele mapeia uma tripla (r,g,b) → (r',g',b'); não é
separável por canal e não funciona sobre um plano isolado. O mesmo vale para
saturação, vivacidade, temperatura e matiz.

### Três saídas possíveis

**A. Converter YUV→RGB→YUV dentro do shader.** No passe de cor, reconstruir o
RGB do pixel, aplicar LUT e ajustes, converter de volta. Problema: o plano Y não
tem os valores de croma correspondentes — precisaria receber os planos U/V como
texturas extras e amostrá-los (com upsampling 4:2:0), e o mesmo para o caminho
inverso. É invasivo nos cinco backends e muda a assinatura dos kernels.

**B. Forçar um formato RGB intermediário quando o LUT estiver ativo.** O pipeline
já tem precedente disso: formatos não suportados nativamente são convertidos para
`YUV444P16LE` (`src/rendering/mod.rs:721,755`). Seria uma conversão análoga para
RGB de alta precisão quando o painel de cor estiver ligado, aceitando o custo de
duas conversões extras por frame. Preserva a precisão (16 bit ou float) e mantém
o shader de cor simples e igual em todos os backends.

**C. Passe separado, depois do warp.** Um segundo kernel que roda sobre o frame
já reunido em RGB, entre a estabilização e o encode. Mais limpo conceitualmente,
mas exige um ponto de execução que hoje não existe — o frame nunca é reunido em
RGB no caminho de GPU.

**Recomendação: B.** É a que reaproveita um mecanismo existente do projeto, não
altera a assinatura dos kernels, e mantém uma única versão da lógica de cor. O
custo é desempenho no export (duas conversões por frame) apenas quando a feature
está ativa — com ela desligada, nada muda.

**Isto precisa da sua decisão antes da Fase 1.**

---

## 1. Build

Não existe `BUILD.md`; o toolchain é o do `justfile` + `_scripts/*.just`.

Ambiente confirmado nesta máquina (documentado na memória do projeto):

```
RUSTUP_HOME=E:\RustToolchain\.rustup
CARGO_HOME=E:\RustToolchain\.cargo
CARGO_BUILD_JOBS=1          # com mais jobs o rustc é morto sem diagnóstico
PATH += E:\RustToolchain\.cargo\bin;C:\Program Files\7-Zip
PKG_CONFIG desabilitado; Git\usr\bin fora do PATH
```

**Armadilha encontrada agora:** sem `7-Zip` no PATH o build falha ao extrair o
mdk-sdk, com erro em `qml-video-rs/build.rs:111` — não é problema do projeto.

Status: build limpo em execução no momento da escrita; resultado registrado na
entrega da fase.

## 2. Pipeline de GPU

### Backends e onde ficam os shaders

| Backend | Fonte | Consumido em |
|---|---|---|
| wgpu (Vulkan/DX12/Metal/GL) | `src/core/gpu/wgpu_undistort.wgsl` | `wgpu.rs:262` |
| OpenCL | `src/core/gpu/opencl_undistort.cl` | `opencl.rs:181` |
| Qt RHI (preview) | `src/qt_gpu/undistort.frag` | `qrhi_undistort.cpp` |
| CPU | `src/core/stabilization/cpu_undistort.rs:233` | fallback |
| rust-gpu (SPIR-V) | `src/core/gpu/stabilize_spirv/src/` | CPU + QSB |

Seleção em runtime: OpenCL primeiro, wgpu como fallback
(`src/core/gpu/mod.rs:152-193`).

### Há duplicação, não abstração

O próprio código avisa, em `src/core/stabilization/mod.rs:102`:

> `// Must be kept in sync with: opencl_undistort.cl, wgpu_undistort.wgsl and qt_gpu/undistort.frag`

Existe um `stabilize_spirv` em rust-gpu que **parece** ser a fonte única, mas não
é: wgpu lê o `.wgsl` manuscrito e OpenCL lê o `.cl` manuscrito. O `stabilize_spirv`
alimenta apenas o caminho CPU e os `.qsb` do Qt.

**Custo real de um passe novo: 3 a 5 cópias da mesma lógica**, mais o campo de
parâmetro em 5 declarações espelhadas da struct.

### Precisão

Todo o processamento interno é **`f32`** em todos os backends. Não há caminho f16
interno — f16 aparece só como formato de armazenamento.

- **Promoção a float:** `wgpu_undistort.wgsl:126-140`; `pixel_formats.rs:10`
  (`to_float_glam`); macro `DATA_CONVERTF` em `opencl.rs:201`.
- **Quantização de volta:** `wgpu_undistort.wgsl:640,644`;
  `opencl_undistort.cl:646-648`; `cpu_undistort.rs:121` (`from_float_glam`).

A escala não é 0..1: `max_pixel_value` vale 255/1023/65535 conforme o bit depth
(`src/core/stabilization/mod.rs:258-259`), exceto em formatos UNORM do wgpu, onde
é forçada a 1.0 (`mod.rs:261-263`). **O passe de cor precisa normalizar por
`max_pixel_value` antes de amostrar o LUT** — o domínio do `.cube` é 0..1.

### Ponto de injeção

O ponto canônico existe e já é usado por outros passes pós-warp:

```rust
// src/core/gpu/stabilize_spirv/src/stabilize.rs:138
pixel = process_final_pixel(pixel, uv, org_out_pos, params, coeffs, drawing, sampler, flags);
```

Implementação em `stabilize_spirv/src/drawing.rs:76-96`. O LUT entraria após o
`remap_colorrange` da linha 78 e antes do drawing overlay da linha 86.

Equivalentes por backend:
- wgpu: `wgpu_undistort.wgsl:642-644` — **e também** o ramo `background_mode == 3`
  em `:636-639`, que tem retorno antecipado.
- OpenCL: `opencl_undistort.cl:650-657`, mais o ramo `:642-647`.
- Qt RHI: `undistort.frag:284-287` e o ramo `:273-282`.

**Cuidado:** esquecer os ramos de retorno antecipado deixaria a cor inconsistente
em parte do frame.

### Parâmetros Rust → shader

Um único uniform buffer com a struct `KernelParams`
(`src/core/stabilization/mod.rs:105-148`, `#[repr(C, packed(4))]` + `bytemuck`).
Upload: `wgpu.rs:469`, `opencl.rs:290`, `qrhi_undistort.rs:45`.

A struct é **espelhada em 5 arquivos** — Rust, `.wgsl`, `.cl`, `.frag` e
`types.rs`. Todo parâmetro novo entra nos cinco.

Existem dois slots `reserved1`/`reserved2` (`mod.rs:144-145` e espelhos) que
**não são lidos em lugar nenhum** — dois floats de graça sem quebrar o layout.

## 3. Texturas 3D

**Não confirmado.** Hoje o projeto não cria nenhuma textura 3D em nenhum backend,
então não há precedente para copiar. O buffer `coeffs` (binding 2,
`wgpu_undistort.wgsl:56`) carrega coeficientes de interpolação e cores de
drawing — um LUT 3D pediria binding próprio, mexendo nos bind group layouts
(`wgpu.rs:317-340`).

Isto fica como **risco aberto**: a viabilidade de textura 3D com sampler linear
em cada backend precisa ser verificada na Fase 2, e o plano de contingência
(textura 2D "tiled", que é como a maioria dos players faz) pode ser necessário
para OpenCL.

## 4. Export: cor e bit depth

### Metadados de cor: hoje é cópia pura da entrada

```rust
// src/rendering/ffmpeg_video.rs
:128  encoder.set_color_range(color_range);          // = frame.color_range() (:105)
:129  encoder.set_colorspace(frame.color_space());
:135  (*encoder.as_mut_ptr()).color_trc = (*frame.as_ptr()).color_trc;
:137  (*encoder.as_mut_ptr()).color_primaries = (*frame.as_ptr()).color_primaries;
```

**Sobrescrever é viável sem fork.** `color_trc` e `color_primaries` já são
escritos por ponteiro cru, porque o wrapper não expõe setters — o mesmo padrão
serve para forçar bt709. Não há hoje nenhum campo em `EncoderParams`
(`ffmpeg_video.rs:37-45`) nem em `RenderOptions` para isso; seria acréscimo.

**O ffmpeg-next NÃO é fork** — `Cargo.toml:60` aponta para a versão 8.1.0 do
crates.io, confirmado em `Cargo.lock:1289-1292`. A especificação supunha um fork;
não é o caso, e isso é boa notícia: nada precisa ser modificado a montante.

Dois defeitos pré-existentes encontrados de passagem:
- `color_trc` é **descartado em videotoolbox** (`ffmpeg_video.rs:134`) — export no
  macOS perde a transfer function.
- O conversor `sws` é fixado em **BT.709** (`ffmpeg_video.rs:340-347`)
  independentemente do espaço real do frame — cor errada em material BT.2020.

### 10-bit funciona

O caminho existe e é exercitado: ProRes/DNxHD/CFHD já forçam `YUV422P10LE`
(`mod.rs:291,302,310`), e o processamento na GPU cobre P010/P210/P410 e
YUV*P10/12/14/16LE (`mod.rs:654-688`). Formatos não suportados caem em
`YUV444P16LE` (`mod.rs:721,755`), preservando 10-bit.

A decisão 2 da especificação (preservar bit depth) é viável.

## 5. Projeto `.gyroflow`

Serialização em `src/core/lib.rs:1255` (`export_gyroflow_data`), leitura em
`:1459`. Não usa struct serde — monta um `serde_json::json!` dinâmico (`:1286`).

Retrocompatibilidade é por leitura defensiva, chave a chave
(`lib.rs:1500-1504`): chave ausente mantém o valor atual.

**O ponto de extensão correto é `util::merge_json` (`lib.rs:1364`)**, que permite
à UI injetar blocos próprios sem tocar no core. Já há precedente nesta mesma
árvore — o painel de áudio externo:

- escrita: `src/ui/App.qml:769` (`"audio_sync": ...` em `getAdditionalProjectData`)
- leitura: `src/ui/menu/ExternalAudio.qml:102` (`loadGyroflow`)
- dispatch: `src/ui/VideoArea.qml:141-149`

O bloco de cor seguiria exatamente esse molde.

## 6. Settings globais

`src/core/settings.rs` — um `HashMap<String, serde_json::Value>` persistido em
`data_dir()/settings.json` (`settings.rs:106`), com escrita debounced de 1 s
(`:118`). Ponte QML em `src/ui/components/Settings.rs:14-19`.

Dois padrões de uso:
- direto: `settings.value(...)` / `settings.setValue(...)`
  (`Advanced.qml:64,78`);
- automático: item `sett` com `property alias` + `settings.init(sett)`
  (`Advanced.qml:14-41`) — auto-salva.

É onde a **pasta de LUTs** deve viver, conforme a decisão 7.

**Limitação:** arrays e objetos não sobrevivem à ponte QVariant→JSON
(`Settings.rs:121-122`). Listas precisam ir como string JSON — o projeto já faz
isso em `Export.qml:73`.

## 7. Efeitos espaciais (Nitidez e Vignette)

**Vignette: viável.** `process_final_pixel` já recebe `out_pos`
(`drawing.rs:76`), a coordenada de saída do pixel, e `params` traz largura e
altura — suficiente para a distância radial normalizada e a correção de aspect
ratio.

**Nitidez: não viável no ponto atual.** O passe é por-pixel e não tem acesso à
vizinhança já processada. Amostrar vizinhos ali significaria refazer o warp para
cada tap do kernel — proibitivo. Precisaria de um passe separado sobre o frame já
estabilizado, que hoje não existe.

Some-se a isto o bloqueador dos planos: nitidez aplicada só no plano Y é
defensável (o olho é mais sensível a luminância), mas é uma decisão de produto,
não um detalhe.

**A Fase 6 precisará de replanejamento**, e o vignette pode ser adiantado para a
Fase 5 sem custo.

## 8. UI

Padrão de painel: `MenuItem` (`src/ui/components/MenuItem.qml`); dez painéis em
`src/ui/menu/`. **Não existe painel de cor hoje** — a seção seria nova.

Componentes prontos que a feature precisa:
- **`SliderWithField.qml`** — slider + campo numérico editável, com `from/to/
  unit/precision/defaultValue` e "Reset value" no menu de contexto. Uso real em
  `Advanced.qml:100-112`. Serve para os treze sliders.
- **`ComboBox.qml`** — usado em `Advanced.qml:57`, `Export.qml:259`. Serve para a
  biblioteca de LUTs.

Comunicação QML → core: `qt_method!` / `qt_property!` em `src/controller.rs`
(exemplo completo em `controller.rs:105` + `ExternalAudio.qml:24`).

**Registro obrigatório de arquivo novo em 3 lugares** (lista manual, esquecer
qualquer um quebra o build ou o runtime):
1. `src/resources_qml.rs:19-28`
2. `src/ui/menu/qmldir`
3. `src/ui/App.qml` (ItemLoader + property alias)

Mais 2 para persistir no projeto: `App.qml:763` e `VideoArea.qml:141-149`.

---

## Riscos, em ordem de gravidade

1. **Planos YUV** (bloqueador acima). Sem resolver, não há LUT 3D.
2. **Nitidez precisa de passe próprio** — a Fase 6 muda de escopo.
3. **Textura 3D sem precedente no projeto** — verificar por backend na Fase 2;
   OpenCL pode exigir emulação 2D tiled.
4. **Cinco cópias da lógica de cor** — todo ajuste novo se multiplica; considerar
   gerar os shaders a partir de uma fonte comum, ou aceitar a duplicação que o
   projeto já convive.
5. **Ramos de retorno antecipado** nos shaders — fácil de esquecer, resulta em
   cor aplicada em parte do frame.
6. **`sws` fixado em BT.709** — defeito pré-existente que pode confundir a
   validação da tag de cor na Fase 7.

## O que muda na especificação

- **`ffmpeg-next` não é fork** — a especificação supunha que fosse. Sobrescrever
  metadados não precisa de mudança a montante.
- **Fase 6 (Nitidez)** precisa de replanejamento; vignette pode subir para a
  Fase 5.
- **Fase 2** ganha um risco: validar textura 3D em cada backend antes de assumir
  que funciona.
