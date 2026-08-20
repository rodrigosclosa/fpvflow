# Estado da feature de sincronização de áudio externo

**Atualizado em:** 2026-08-20
**Especificação:** [PROMPT-MESTRE.md](PROMPT-MESTRE.md) · **Reconhecimento:** [recon.md](recon.md)

---

## Resumo

| Fase | Escopo | Estado |
|---|---|---|
| 0 | Reconhecimento + build limpo | ✅ |
| 1 | Decode + waveform na timeline | ✅ |
| 2 | Slider manual + persistência | ✅ |
| 3 | Export com muxing + preservação de formato | ✅ validada com arquivo real |
| 4 | Trim respeitando o áudio | ✅ |
| 5 | Auto-sync por correlação | ✅ dois métodos; falta medir o erro com material real |

**50 testes passando** no `gyroflow-core` (+ 102 no crate raiz).
`cargo build --release` verde. **A feature já foi usada num export real**, com
vídeo de DJI O4P 4K e WAV do DJI Mic.

---

## ▶ PRÓXIMA TAREFA: áudio na prévia do player

**Pedido do Rodrigo, já aprovado e iniciado.** Sem ouvir o áudio junto do vídeo,
o ajuste fino do offset é feito às cegas — só pela waveform.

### O que já foi levantado

O **MDK** (motor de vídeo do Gyroflow) suporta áudio externo nativamente:

```c
// mdk-sdk/include/mdk/c/Player.h:168
void (*setMediaForType)(struct mdkPlayer*, const char* url, MDK_MediaType type);
/* Set individual source for type, e.g. audio track file.
   MUST be after main media setMedia(url). */
```

O obstáculo: o Gyroflow não fala com o MDK direto. Entre eles está o
`qml-video-rs`, que expõe apenas `muted` e `volume` — `setMediaForType` **não
está exposto**.

### Caminho combinado (opção A, aprovada pelo Rodrigo)

1. **O repositório já está clonado** em `E:\Documentos - Rodrigo Sclosa\Projetos\gyroflow\qml-video-rs`,
   no rev exato que o projeto usa (`7eb9292`).
2. Adicionar três métodos seguindo o padrão de `setMuted`, que atravessa
   quatro arquivos:
   - `src/cpp/MDKPlayer.h` — declaração (ver linha 38, `setMuted`)
   - `src/cpp/MDKPlayer.cpp` — implementação (ver linha 176)
   - `src/lib.rs` — ponte C++/Rust
   - `src/video_item.rs` — propriedade QML (ver linha 168)

   Sugestão de API: `setExternalAudio(url)`, `clearExternalAudio()` e um
   `audioOffset` — ou aplicar o offset gravando um WAV temporário já alinhado,
   se o MDK não permitir deslocar a trilha.
3. Publicar o fork no GitHub do Rodrigo e trocar a dependência em
   `gyroflow/Cargo.toml:57`. A linha 58 já tem a variante `path = "../qml-video-rs"`
   comentada, útil para desenvolver antes de publicar.
4. Ligar ao painel: quando uma trilha é importada, chamar `setExternalAudio`; ao
   remover, `clearExternalAudio`.

---

## Como testar o que já existe

O binário fica em `gyroflow\target\release\gyroflow.exe` e **não abre com duplo
clique** — precisa das DLLs de `ext\`:

```powershell
$r="E:\Documentos - Rodrigo Sclosa\Projetos\gyroflow\gyroflow"
$env:PATH="$r\ext\6.7.3\msvc2019_64\bin;$r\ext\ffmpeg-8.1-windows-desktop-vs2026-gpl-lite\bin;$r\ext\vcpkg\installed\x64-windows\bin;"+$env:PATH
& "$r\target\release\gyroflow.exe"
```

O pacote autônomo (`_deployment\_binaries\win64\Gyroflow.exe`, abre com duplo
clique) **está desatualizado** — foi gerado antes das últimas correções.
Regerar com `just deploy` quando quiser distribuir. Leva ~13 min e precisa ser
disparado desacoplado, senão o executor de tarefas o mata aos 10 min.

---

## Onde a feature aparece na interface

Painel próprio **"External audio"**, com ícone de som, em `src/ui/menu/ExternalAudio.qml`,
logo abaixo de "Dados de movimento". Antes de importar, só o botão de importar
aparece; o resto surge com a trilha carregada:

importar · formato detectado · caminho · **Auto-sync** com confiança · slider de
offset (ms + frames) · selo de preservação · checkbox de preservar formato ·
seção Avançado (banda automática/fixa, passa-alta do gyro) · remover.

A waveform é uma lane na timeline, abaixo do gráfico do gyro
(`Timeline.qml`, `TimelineAudioWaveform.rs`).

---

## Os dois métodos de auto-sync

A escolha é automática, pela taxa de amostragem do gyro (limite de 300 Hz):

**Banda das pás** (gyro ≥ 300 Hz) — o método da especificação. STFT, energia na
banda 150–900 Hz, correlação com a vibração lida pelo giroscópio. Preciso ao
milissegundo.

**Início do movimento** (gyro < 300 Hz) — para câmeras que só entregam
quaternions integrados. O DJI O4P dá ~32 quaternions em 48 s (0,66 Hz): por
Nyquist, a vibração das hélices não existe nesse sinal. Alinha pelo instante da
decolagem, que aparece como salto de energia nos dois lados. Precisão de
segundos, não de milissegundos.

A interface informa qual foi usado.

---

## Validações já feitas

- **Export com preservação de formato**: `ffprobe` no arquivo final confirmou
  `pcm_f32le`, 48 kHz, 2 canais, duração igual à do vídeo.
- **Offset**: transiente gravado em t=2,0 s aparece em 2,001 s com offset 0 e em
  1,001 s com offset +1,0 s. Erro de um sample.
- **Conflito float × MP4**: avisa antes de exportar, sem conversão silenciosa.
- **Roundtrip**: erro máximo < 1e-6 comparando amostra a amostra.
- **Uso real**: DJI O4P 4K 59,94 fps + WAV do DJI Mic, offset manual de
  −25,599 s, com trim. Exportou corretamente.

---

## Bugs encontrados testando (todos corrigidos)

Nenhum era visível na compilação:

1. **WAV float classificado como comprimido** — o symphonia não preenche
   `sample_format` para PCM; a informação está no `CodecType`. O material seria
   exportado como AAC silenciosamente.
2. **Crash em áudio estéreo** — `frame::Audio::plane_mut::<f32>(0)` é
   dimensionado em frames, não amostras.
3. **Estouro de índice no render** — `ost_time_bases` tem o tamanho dos streams
   de entrada, e a saída ganha um a mais com o áudio externo.
4. **Painel invisível (1ª tentativa)** — conteúdo posto dentro de um `Label`,
   que é um `Grid` de 2 colunas.
5. **Painel não encontrado (2ª tentativa)** — `src/resources_qml.rs` é uma lista
   **manual** dos QML embutidos no binário. Todo arquivo `.qml` novo precisa ser
   adicionado lá, além do `qmldir`.
6. **TypeError ao salvar projeto** — o `ItemLoader` é assíncrono e `item` é
   `null` nos primeiros segundos.

---

## O que ainda falta

1. **Áudio no player** — a próxima tarefa, detalhada acima.
2. **Medir o erro do auto-sync com material real.** O Rodrigo testou o
   auto-sync antes da correção dos quaternions e recebeu "Not enough data to
   sync". Agora deve funcionar pelo método de onset. Falta comparar o offset
   detectado com o real (ele achou −25,599 s manualmente) e anotar o erro.
3. **Testar prioridade sobre áudio embutido.** O clipe do O4P não tem áudio
   próprio (`audio_stream_num: 0`), então a regra "externo substitui embutido"
   nunca foi exercitada.
4. **Regerar o pacote de deployment** depois de tudo estabilizado.

---

## Decisões de arquitetura

1. **`export.rs` dividido**: lógica pura em `src/core/audio/export.rs`, encode e
   mux em `src/rendering/audio_export.rs`. `gyroflow-core` não depende de ffmpeg
   e não deve passar a depender.
2. **Decode com `symphonia`**, não com o ffmpeg do projeto — mesma razão.
3. **Conflito float×MP4 sugere `.mov`**, que já é o container de
   ProRes/DNxHD/CineForm.
4. **O fork do `ffmpeg-next` não precisou ser modificado.** `add_stream`,
   `avcodec_alloc_context3` e `encoder().audio()` são públicas e já usadas pelo
   `AudioTranscoder` (`ffmpeg_audio.rs:17-46`).
5. **`trim_ranges` são normalizados 0..1**, não segundos.
6. **`IMUData` usa `[f64;3]`** (`telemetry-parser` rev `77a3b81`).

---

## Riscos em aberto

- **Deriva de clock** entre gravador e câmera em clipes longos. Só aparece com
  material real; a saída seria estimar também um fator de resample.
- **Gimbal mecânico** atenua a vibração das pás e derruba a confiança do
  auto-sync. A interface sinaliza confiança < 30%.
- **Áudio desativado quando a velocidade do vídeo muda** (comportamento
  pré-existente do Gyroflow). O áudio externo herda a limitação.
- **Onset depende de haver um evento comum.** Se o áudio começar a gravar depois
  da decolagem, não há o que casar.
