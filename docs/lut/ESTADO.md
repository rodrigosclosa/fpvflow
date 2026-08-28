# Estado da feature de LUT e painel de cor

**Atualizado em:** 2026-08-27
**Branch:** `fpvflow` (fork FPVFlow, repo `rodrigosclosa/fpvflow`)

> Leia [recon.md](recon.md) antes de tocar em código. Ele tem os `arquivo:linha`
> de cada ponto de injeção e o bloqueador que define o desenho da feature.

---

## Onde parou

| Fase | Estado |
|---|---|
| 0 — Reconhecimento | **Concluída.** `recon.md` + build limpo passando |
| 1 — Parser `.cube` | **Concluída.** 13 testes, clippy limpo |
| 2 — Upload GPU | **Concluída** nos quatro caminhos: wgpu, OpenCL, CPU e Qt RHI |
| 3 — Shader do LUT | não iniciada |
| 4 — Biblioteca de LUTs | não iniciada |
| 5 — Ajustes por-pixel | não iniciada |
| 6 — Nitidez e vignette | não iniciada — **precisa replanejamento**, ver abaixo |
| 7 — Export | não iniciada |

Commits: `06e2bc56` (parser + recon), `edcf3257` (correção do build.rs),
`2072ca1f` (layouts de textura), `bb518504` (estado), + o LUT no wgpu.

## O que existe

```
src/core/color/
  mod.rs            # documenta a ordem da pipeline de cor
  lut/
    mod.rs          # Lut, LutData, sample() trilinear, to_rgba_f32()
    parser.rs       # .cube -> Lut, 9 testes
    gpu.rs          # LutTexture nos dois layouts, 4 testes
docs/lut/
  recon.md          # Fase 0
  ESTADO.md         # este arquivo
```

Registrado em `src/core/lib.rs:6` (`pub mod color;`).

## O bloqueador que define tudo

**A GPU processa planos YUV, um por vez** (`src/rendering/mod.rs:735-738`; o
índice vai em `mod.rs:616`). Um LUT 3D precisa dos três canais do mesmo pixel, e
em 4:2:0 o plano Y roda sozinho enquanto U/V têm metade da resolução.

**Decisão do Rodrigo:** converter para um formato RGB intermediário quando a
feature estiver ativa, reusando o mecanismo que o projeto já tem
(`convert_pixel_format`, `mod.rs:755`).

**Formato escolhido: `RGBA16`** (via `Pixel::RGBA64BE`) — é o único RGB de alta
precisão **já no dispatch** (`mod.rs:717`) e com formato wgpu válido
(`Rgba16Uint`). Detalhe que morde: `Rgba16Uint` **não é unorm**
(`pixel_formats.rs:187`), então o shader recebe valores 0..65535, não 0..1 — tem
de normalizar por `max_pixel_value` antes de indexar o LUT.

Custo: duas passagens de `sws` por frame, em CPU
(`ffmpeg_video_converter.rs:50,54`). Em 4K vira provavelmente o gargalo do
export. Só acontece com a feature ligada.

## O que o wgpu já tem

- `init_lut_texture` (`gpu/wgpu_interop.rs:44`) cria a textura 3D `Rgba32Float`.
- Binding **6** no caminho de render e **7** no de buffer — os dois layouts
  divergem porque o de buffer já usa o 6 para a saída (`wgpu.rs:317-345`).
- `apply_color_lut` no `.wgsl`, chamado nos **dois** pontos de saída que carregam
  pixel de imagem. Os outros dois retornos (`:621,634`) devolvem a cor de fundo
  escolhida pelo usuário e **não devem** ser graduados.
- `WgpuWrapper::set_lut` troca o LUT com um `write_texture`, sem recriar nada.

### Três decisões que mudaram o desenho

**1. Não existe sampler neste projeto.** A recon supôs que um sampler linear daria
a interpolação trilinear de graça. Não dá: todas as texturas são ligadas com
`filterable: false` e lidas com `textureLoad` (`wgpu_undistort.wgsl:128`). O
shader interpola à mão, espelhando `Lut::sample`. **O layout `Volume3D` perdeu
sua vantagem sobre o `Tiled2D`** — a escolha entre eles agora é só sobre qual
backend consegue amostrar textura 3D.

**2. A textura é alocada no tamanho máximo (65³) e nunca redimensiona.** O bind
group é montado uma vez, e refazê-lo exigiria reter buffers que o código descarta
de propósito (`buf_coeffs`, `wgpu.rs:308`). Trocar de LUT vira um `write_texture`.
Custo: ~4,4 MB fixos. LUTs maiores que 65 são **reduzidos** — o parser aceita até
256 (`parser.rs:131`), e essa direção perde detalhe.

**3. O LUT está sempre presente, com identidade quando nenhum foi carregado.** A
chave de cache do pipeline (`stabilization/mod.rs:355`) **não inclui qual LUT
está carregado**, então um binding que aparece e some exigiria uma segunda
variante de pipeline. Com a identidade, o custo de "desligado" é um fetch de
textura.

## Os outros três caminhos

**OpenCL** (tentado **primeiro** em `gpu/mod.rs:152-193`, então é o que roda na
maioria das máquinas): o LUT vai como **buffer de float**, não `image3d_t`.
Suporte a imagem 3D é opcional em OpenCL e exigiria checagem de capacidade com
fallback tiled; como todo o resto do kernel já entra por ponteiro `__global` e a
interpolação é explícita de qualquer jeito, o buffer evita a questão sem custo.

**CPU**: o fallback roda o próprio undistort (`undistort_image_cpu`), não o
`stabilize_spirv` — então chama `Lut::sample` direto, sem uma quarta
reimplementação. Aplicado **antes do `remap_colorrange`**, que comprime para
limited range enquanto o domínio do LUT é full range; a ordem não é
intercambiável.

**Qt RHI**: único backend com **sampler linear de verdade**, então a
interpolação trilinear é do hardware. Isso exige a correção de meio texel
(`apply_color_lut` no `.frag`): amostrar em 0 ou 1 cai na borda do texel, não no
centro, e corta as pontas da tabela. Trabalha em 0..1, sem `max_pixel_value`.

### O que mudou no build por causa do Qt

Os 22 `.qsb` são binários pré-compilados e commitados, num container
**versionado**: Qt 6.4 lê versão 6, Qt 6.7 escreve versão 9. Um único conjunto é
embarcado para todas as plataformas, então o `QtVersion` do `_scripts/common.just`
foi fixado em **6.7.3** — o upstream ainda mandava Linux e macOS x86_64 para
6.4.3, mas o CI deste fork já forçava 6.7.3 no macOS. Recompilar:
`cd src/qt_gpu/compiled && bash compile_shaders.sh`.

`LUT_SIZE` é repetido em C++ (`qrhi_undistort.cpp`) porque um bloco `cpp!` não
enxerga constante de Rust; há um `const _: () = assert!(...)` em
`qrhi_undistort.rs` para travar a divergência em tempo de compilação.

**Init falha se o dispositivo não tiver textura 3D.** Isso é só OpenGL ES 2.0, e
a alternativa é pior: `controller.rs:1112` retorna `true` sem cair para outro
backend, então uma falha silenciosa daria prévia em branco em vez de log.

## Fase 3 — UI e o bloqueador YUV

**Painel `src/ui/menu/ColorLut.qml`**, entre Estabilização e Exportação. Carrega
`.cube`, mostra tamanho/título, o caminho, e o **erro de parse** quando o arquivo
não serve — a mensagem nomeia a linha, que é o que torna um download truncado
reconhecível.

Registrado nos **5** pontos obrigatórios: `resources_qml.rs`, `menu/qmldir`,
`App.qml` (ItemLoader + alias), `App.qml:getAdditionalProjectData` e
`VideoArea.qml` (dispatch de `loadGyroflow`). O projeto guarda a **URL interna**,
não o caminho legível — no Android os dois não são intercambiáveis.

Controller: `load_color_lut`, `load_color_lut_url`, `clear_color_lut`,
`get_color_lut_url/_path/_info` e o sinal `color_lut_changed(ok, error)`. O
`get_color_lut_info` devolve **JSON**, não frase pronta, como o
`get_external_audio_info` — a tradução vive no QML, onde o `qsTr` alcança.

### O bloqueador YUV, resolvido — e menor do que eu pensava

**A prévia nunca teve o problema.** Ela decodifica para **`RGBA8`**
(`controller.rs:1068`), então sempre viu RGB completo. O bloqueador valia só para
a **exportação**.

No `rendering/mod.rs`, quando há LUT carregado e o formato ainda não é RGB, o
frame passa por **`RGBA64BE`** — o único RGB de alta precisão já tratado ali, o
que evita achatar material 10/12 bit para 8. Custa duas passagens de `sws` por
frame, e **só acontece com LUT carregado**.

Detalhe que quase virou bug: `planes` é montado **uma vez** (guardado por
`planes.is_empty()`), enquanto o dispatch de formato roda **a cada frame**. Os
dois precisam concordar, então leem a mesma flag `needs_rgb_for_lut` em vez de
recalcular a condição.

## Próximo passo

**Fase 4 — biblioteca de LUTs** (pasta global nas settings) e o **slider de
intensidade**, que a especificação pede e ainda não existe: hoje o LUT é aplicado
a 100%.

Depois, Fases 5–7 (ajustes por-pixel, vignette, tag de cor no export). A **Fase 6
segue precisando de replanejamento** — nitidez não cabe no ponto de injeção
atual, conforme o risco 1.

### Como isto é testado sem GPU

`cargo test --lib gpu::wgpu` valida o WGSL com o `naga` (já é dependência) nas
quatro variantes, e confere os índices de binding contra o que o layout declara.
Verifiquei que os dois testes **falham** quando o índice é alterado de propósito
— antes deles, um erro de binding só apareceria como falha de validação na
máquina do usuário.

## Riscos, em ordem

1. **Nitidez não cabe no ponto de injeção atual.** `process_final_pixel`
   (`stabilize_spirv/src/drawing.rs:76`) é por-pixel; amostrar vizinhos exigiria
   refazer o warp por tap. **A Fase 6 precisa de um passe separado** — ou de
   aceitar nitidez só no plano Y. Vignette funciona, e pode subir para a Fase 5.
2. **Preview e export divergem.** O preview padrão (Qt RHI) tem shader GLSL
   próprio, em **RGBA8** (`qrhi_undistort.rs:8,39`; textura em
   `qrhi_undistort.cpp:70`), enquanto o export usaria RGBA16. O LUT precisa ser
   escrito duas vezes, e **a precisão que se vê não é a que se exporta**.
3. **3 a 5 cópias da lógica de cor.** O código avisa em `mod.rs:102`. Todo ajuste
   novo se multiplica; a struct `KernelParams` é espelhada em 5 arquivos
   (`mod.rs:105-148`, `.wgsl`, `.cl`, `.frag`, `types.rs`). Há dois slots
   `reserved1`/`reserved2` livres, não lidos em lugar nenhum.
4. **Ramos de retorno antecipado nos shaders** (`wgpu_undistort.wgsl:639`,
   `opencl_undistort.cl:647`, background_mode 3). Esquecer um deixa parte do
   frame sem o LUT.
5. **`sws` fixado em BT.709** (`ffmpeg_video.rs:340-347`) independentemente do
   espaço real — defeito pré-existente que pode confundir a validação da Fase 7.

## Correções à especificação original

- **`ffmpeg-next` não é fork** — é o crate oficial 8.1.0 (`Cargo.toml:60`,
  `Cargo.lock:1289`). Sobrescrever os metadados de cor para bt709 já é possível
  pelo padrão de ponteiro cru do arquivo (`ffmpeg_video.rs:135,137`), sem
  mudança a montante.
- **10-bit funciona** (`mod.rs:654-688`); a decisão de preservar bit depth é
  viável.
- **Fase 6 muda de escopo** (item 1 dos riscos).

## Armadilhas de build encontradas aqui

- **`cpp!` com metadados velhos**: `error: This cpp! macro is not found in the
  library's rust-cpp metadata`. O `build.rs` declarava `rerun-if-changed` sem
  incluir `src/gyroflow.rs`, o que desliga o padrão do Cargo de observar o
  pacote. Corrigido em `edcf3257`. Se reaparecer noutro arquivo com `cpp!`:
  `cargo clean -p gyroflow`.
- **Comentários `//` dentro de um bloco `cpp!`** quebram o parser textual do
  `rust-cpp` — mantenha-os fora.
- **7-Zip precisa estar no PATH**, senão o build falha ao extrair o mdk-sdk.
- **`just build` leva ~7 min** e é morto pelo executor aos 10 — rodar
  desacoplado.
