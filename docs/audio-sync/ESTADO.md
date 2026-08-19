# Estado da feature de sincronização de áudio externo

**Atualizado em:** 2026-08-19
**Especificação:** [PROMPT-MESTRE.md](PROMPT-MESTRE.md) · **Reconhecimento:** [recon.md](recon.md)

---

## Resumo

| Fase | Escopo | Estado |
|---|---|---|
| 0 | Reconhecimento + build limpo | ✅ concluída |
| 1 | Decode + waveform na timeline | ✅ concluída |
| 2 | Slider manual + persistência | ✅ concluída |
| 3 | Export com muxing + preservação de formato | ✅ concluída e **validada com arquivo real** |
| 4 | Trim respeitando o áudio | ✅ concluída |
| 5 | Auto-sync por correlação | ✅ concluída (falta material real de drone) |

**149 testes passando** (47 no `gyroflow-core`, 102 no crate raiz).
`cargo build --release` verde; binário verificado executando.
`clippy` sem avisos no módulo `audio` — os 7 erros que ele aponta no core são
pré-existentes, em `cpu_undistort`, `splines`, `lens_profile` e `filesystem`.

---

## O que foi verificado rodando de verdade

Não só compila: o pipeline foi exercitado com arquivos reais, e o resultado
conferido com `ffprobe`.

### Exportação com áudio 32-bit float preservado

Vídeo H.264 de 3 s + WAV IEEE float estéreo de 5 s, saída `.mov`:

```
codec_name=pcm_f32le    sample_fmt=flt
sample_rate=48000       channels=2       duration=3.000000
```

Float entra, float sai. Taxa e contagem de canais originais intactas, e o áudio
com a mesma duração do vídeo — das 480.000 amostras do arquivo, 288.000 foram
escritas e o excedente cortado.

### Offset medido no arquivo final

O WAV de teste tem um transiente em `t = 2,0 s`. No vídeo exportado:

| offset pedido | transiente aparece em | deslocamento |
|---|---|---|
| 0,0 s | 2,001 s | — |
| +1,0 s | 1,001 s | **1,000 s** |

Erro de um único sample, vindo do arredondamento. A convenção
`t_audio = t_video + offset` está correta ponta a ponta.

### Conflito float × MP4

Exportando o mesmo material para `.mp4`, o log traz **antes** de escrever:

```
Áudio externo: PCM (f32le) não cabe em .mp4.
Troque a saída para .mov para preservar o formato. Exportando com AAC.
```

Nenhuma conversão silenciosa — o requisito central da decisão 7.

### Roundtrip sem perda

O teste `conteudo_do_audio_sobrevive_ao_encode` escreve uma trilha, relê o
arquivo e compara amostra a amostra: erro máximo **< 1e-6**. Se houvesse
quantização para inteiro em algum ponto do caminho, o erro seria ordens de
grandeza maior.

---

## Bugs encontrados ao testar com arquivos reais

Três defeitos que a compilação não pegaria, e que teriam aparecido no primeiro
uso:

1. **WAV float classificado como formato comprimido.** O symphonia não preenche
   `sample_format` para streams PCM — a informação está no `CodecType`. O
   material 32-bit float seria exportado como AAC, silenciosamente, exatamente
   o que a decisão 7 proíbe. Corrigido em `decode.rs` com `map_codec_type()` e
   travado por um teste de regressão.

2. **Crash em áudio estéreo.** `frame::Audio::plane_mut::<f32>(0)` é
   dimensionado em *frames*, não em amostras, e devolvia metade do buffer
   necessário. Mono passava; qualquer trilha de 2 canais estouraria o índice.

3. **Estouro de índice no render.** `ost_time_bases` tem o tamanho do número de
   streams de *entrada*, mas a saída ganha um stream a mais quando o áudio
   externo é embutido. O render morria antes de escrever qualquer frame.

---

## O que ainda depende de você

**Validação da Fase 5 com material real.** O auto-sync foi verificado com sinais
sintéticos (recupera offsets conhecidos, positivos e negativos, com erro
< 30 ms), mas nunca viu a vibração de um drone de verdade. O teste que falta:

1. Gravar um clipe com o mic externo ligado, dando uma palma ou claquete na
   frente do drone.
2. Importar o vídeo e o áudio, clicar em **Auto-sync audio**.
3. Comparar o offset detectado com o real e anotar o erro em ms.

Se a banda automática travar no vento em vez das hélices, a confiança exibida
cai — nesse caso, desligar `auto_band` e usar a banda fixa (150–900 Hz).

**Validação da interface.** Os testes cobrem a lógica, não o desenho: vale
conferir a waveform na timeline, o slider e o selo de preservação de formato
com o programa aberto.

---

## Decisões tomadas durante a execução

Registradas também no anexo do [PROMPT-MESTRE.md](PROMPT-MESTRE.md).

1. **`export.rs` dividido em dois.** Lógica pura (buffer, offset, trim,
   silêncio) em `src/core/audio/export.rs`, testável sem ffmpeg; encode e mux em
   `src/rendering/audio_export.rs`. `gyroflow-core` não depende de ffmpeg e não
   deve passar a depender.

2. **Decode com `symphonia`, não com o ffmpeg do projeto.** Mesma razão.
   É Rust puro, já estava no `Cargo.lock` via rodio, e cobre WAV/PCM float além
   de AAC/MP3/FLAC.

3. **Container sugerido no conflito float×MP4: `.mov`.** Já é o container de
   ProRes/DNxHD/CineForm no seletor de exportação, e `App.qml:729` já traz a
   mensagem oficial recomendando `.mov`.

4. **O fork do `ffmpeg-next` não precisou ser modificado.** `add_stream`,
   `avcodec_alloc_context3` e `encoder().audio()` são as mesmas APIs públicas
   que o `AudioTranscoder` já usa (`ffmpeg_audio.rs:17-46`). Resolve o item 3(a)
   da Fase 0 e elimina o risco de aumento de escopo.

5. **`IMUData` usa `[f64;3]`**, não `[f32;3]` (`telemetry-parser` rev `77a3b81`,
   `src/util.rs:289-294`).

6. **A taxa de amostragem do gyro é derivada dos timestamps** dentro do
   `gyro_envelope`. Para alinhar as séries não é preciso reamostrar o gyro: a
   grade temporal vem do áudio.

---

## Riscos ainda em aberto

- **Deriva de clock.** Gravador e câmera têm clocks independentes; em clipes
  longos um único offset pode não bastar. Só aparece com material real. Se
  acontecer, a saída é estimar também um fator de resample.
- **Gimbal mecânico.** Se o sensor estiver isolado, a vibração das pás chega
  atenuada ao gyro e a confiança do auto-sync cai. A interface já sinaliza
  confiança abaixo de 30%.
- **Áudio desativado quando a velocidade do vídeo muda** (`rendering/mod.rs`,
  comportamento pré-existente). O áudio externo herda essa limitação.
- **Dois caminhos de áudio em paralelo.** O trim de precisão de sample não
  reusa o `AudioTranscoder` — montamos o buffer inteiro. É bom para a precisão,
  mas os dois caminhos precisam ser mantidos.
