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
| 2 — Upload GPU | **Metade.** Payload pronto; falta criar a textura em cada backend |
| 3 — Shader do LUT | não iniciada |
| 4 — Biblioteca de LUTs | não iniciada |
| 5 — Ajustes por-pixel | não iniciada |
| 6 — Nitidez e vignette | não iniciada — **precisa replanejamento**, ver abaixo |
| 7 — Export | não iniciada |

Commits: `06e2bc56` (parser + recon), `edcf3257` (correção do build.rs),
`2072ca1f` (layouts de textura).

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

## Próximo passo (Fase 2, segunda metade)

Criar a textura de fato, por backend:

- **wgpu**: padrão em `wgpu_interop.rs:50-64`; binding novo nos layouts em
  `wgpu.rs:317-340` (hoje vão até o binding 5).
- **OpenCL**: verificar `image3d_t` com sampler linear; se não houver, usar o
  layout tiled que já está pronto.
- **Qt RHI**: shader GLSL próprio (`src/qt_gpu/undistort.frag`), com os `.qsb`
  recompilados por `src/qt_gpu/compiled/compile_shaders.sh`.

Validação da fase: carregar um `.cube` e o log confirmar a textura criada no
tamanho certo, sem erro de validação da API. **A imagem ainda não muda.**

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
