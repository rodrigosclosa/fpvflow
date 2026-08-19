# Estado da feature de sincronização de áudio externo

**Atualizado em:** 2026-08-19
**Especificação:** [PROMPT-MESTRE.md](PROMPT-MESTRE.md) · **Reconhecimento:** [recon.md](recon.md)

---

## Resumo

| Fase | Escopo | Estado |
|---|---|---|
| 0 | Reconhecimento + build limpo | ✅ concluída |
| 1 | Decode + waveform na timeline | ✅ código completo |
| 2 | Slider manual + persistência | ✅ código completo |
| 3 | Export com muxing + preservação de formato | 🟡 **parcial** — falta o passo final |
| 4 | Trim respeitando o áudio | ✅ lógica completa (depende da Fase 3) |
| 5 | Auto-sync por correlação | ✅ código completo |

**40 testes unitários passando.** `cargo build --release` verde. `clippy` sem
avisos no módulo `audio` (os 7 erros que ele aponta no core são pré-existentes,
em `cpu_undistort`, `splines`, `lens_profile` e `filesystem`).

**Nenhuma fase foi validada com material real.** Ver "O que falta" abaixo.

---

## O que falta

### 1. Ligar o encoder ao pipeline de exportação (Fase 3)

É o único item de **código** pendente, e sem ele o áudio externo não chega ao
arquivo exportado.

Tudo o que ele precisa já existe e compila:

- `gyroflow_core::audio::export::build_from_trim_ranges` monta o buffer já
  alinhado e recortado.
- `gyroflow_core::audio::export::check_format_compatibility` decide o codec e
  detecta o conflito float×MP4.
- `rendering::audio_export::ExternalAudioEncoder` cria o stream, codifica e
  escreve os pacotes.

Falta chamá-los de dentro do `FfmpegProcessor`. Os pontos de enxerto:

| Onde | O quê |
|---|---|
| `ffmpeg_processor.rs:334-343` | Quando houver áudio externo, **não** criar o `AudioTranscoder` do stream de entrada — o externo tem prioridade (decisão 5 da especificação). |
| `ffmpeg_processor.rs:~350` | Instanciar o `ExternalAudioEncoder` depois dos streams de vídeo, mesmo quando o arquivo de entrada não tem áudio nenhum. |
| `ffmpeg_processor.rs:~509` | Chamar `write_all` com o buffer e, ao fim, `finish()` para drenar o encoder. |
| `render_queue.rs:79-89` | Campos de áudio externo no `RenderOptions`, para que o buffer chegue ao processador. |

**Por que não foi feito:** o fluxo de escrita de pacotes do `FfmpegProcessor`
intercala vídeo e áudio com controle de timestamps (`FrameTimestamps`), e
inserir um stream que não vem do input exige acertar a ordem de escrita e o
`interleaved` do muxer. Escrever isso sem conseguir exportar um vídeo de teste
produziria código plausível e não verificado — exatamente o que a instrução de
"nunca invente APIs" existe para evitar. O caminho está mapeado acima e a parte
difícil (o encoder sem stream de entrada) já está pronta e compilando.

**O que já está decidido:** o fork do `ffmpeg-next` **não** precisa ser
modificado. `add_stream`, `avcodec_alloc_context3` e `encoder().audio()` são as
mesmas APIs públicas que o `AudioTranscoder` já usa (`ffmpeg_audio.rs:17-46`).
Isso resolve a dúvida do item 3(a) da Fase 0 e elimina o risco de aumento de
escopo que o prompt mestre antecipava.

### 2. Validação manual

Nada abaixo pode ser verificado sem os arquivos e a interface — é a parte que
depende de você:

- **Fase 1** — importar um WAV 32-bit float estéreo e ver a waveform na
  timeline, redesenhando ao dar zoom. Conferir no painel que o formato
  detectado aparece como "32-bit float".
- **Fase 2** — ajustar o offset, salvar o projeto, fechar, reabrir: a waveform
  volta na posição ajustada.
- **Fase 3** — depois do item 1 acima: exportar e conferir com `ffprobe` que a
  trilha saiu como `pcm_f32le`, com os canais e o sample rate originais.
  Confirmar também o aviso no caso float + `.mp4`.
- **Fase 4** — definir trim (inclusive múltiplos ranges) e conferir que o áudio
  acompanha o corte sem drift.
- **Fase 5** — gravar uma claquete/palma na frente do drone com o mic ligado,
  rodar o auto-sync e comparar o offset detectado com o real. **Reportar o erro
  medido em ms.** Se a banda automática pegar vento em vez das pás, desligar
  `auto_band` e usar a banda fixa.

---

## Decisões tomadas durante a execução

Registradas também no anexo do [PROMPT-MESTRE.md](PROMPT-MESTRE.md).

1. **`export.rs` dividido em dois.** A lógica pura (buffer, offset, trim,
   silêncio) ficou em `src/core/audio/export.rs`, testável sem ffmpeg; o encode
   e o mux em `src/rendering/audio_export.rs`. Motivo: `gyroflow-core` não
   depende de ffmpeg e não deve passar a depender.

2. **Decode com `symphonia`, não com o ffmpeg do projeto.** Mesma razão: manter
   o core livre de ffmpeg. `symphonia` é Rust puro, já estava no `Cargo.lock` via
   rodio, e cobre WAV/PCM float além de AAC/MP3/FLAC.

3. **Container sugerido no conflito float×MP4: `.mov`.** Já é o container de
   ProRes/DNxHD/CineForm no seletor de exportação, e `App.qml:729` já traz a
   mensagem oficial recomendando `.mov`. MKV segue funcionando para quem digitar
   a extensão, mas não foi exposto na UI.

4. **`IMUData` usa `[f64;3]`**, não `[f32;3]` (verificado em `telemetry-parser`
   rev `77a3b81`, `src/util.rs:289-294`). O áudio chega em `f32`: a conversão é
   explícita no `gyro_envelope`.

5. **A taxa de amostragem do gyro é derivada dos timestamps** dentro do
   `gyro_envelope`, já que não há campo pronto no core. Para alinhar as séries
   não é preciso reamostrar o gyro: a grade temporal vem do áudio.

---

## Riscos ainda em aberto

- **Deriva de clock.** Gravador e câmera têm clocks independentes; em clipes
  longos um único offset pode não bastar. Só aparece em teste com material real.
  Se acontecer, a saída é estimar também um fator de resample.
- **Gimbal mecânico.** Se o sensor estiver isolado, a vibração das pás chega
  atenuada ao gyro e a confiança do auto-sync cai. A UI já sinaliza confiança
  abaixo de 30%.
- **Áudio desativado quando a velocidade do vídeo muda** (`rendering/mod.rs:446`,
  comportamento pré-existente). O áudio externo herda essa limitação.
- **O trim de precisão de sample não reusa o `AudioTranscoder`.** Como montamos
  o buffer inteiro, o caminho é outro — o que é bom para a precisão, mas
  significa que os dois caminhos de áudio precisam ser mantidos em paralelo.
