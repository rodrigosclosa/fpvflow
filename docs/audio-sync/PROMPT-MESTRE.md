# Prompt mestre — Feature de sincronização de áudio externo no Gyroflow

> **Especificação persistente da feature.** Este documento é a fonte da verdade para todas as
> fases. Foi salvo no repositório em 2026-08-19 porque vivia apenas no prompt da sessão e se
> perdeu quando a sessão terminou. Não editar o conteúdo normativo abaixo sem decisão explícita
> do Rodrigo.

---

## 0. Contexto do projeto (leia antes de tocar em código)

Você é um engenheiro Rust/Qt trabalhando num **fork do Gyroflow** (https://github.com/gyroflow/gyroflow). O Gyroflow estabiliza vídeo usando dados de giroscópio. O core é em **Rust**; a UI é em **QML/Qt 6** via `qmetaobject-rs`; o vídeo/mux usa um fork próprio de ffmpeg (`ffmpeg-next`).

**Objetivo da feature:** permitir importar uma trilha de áudio externa (gravada com DJI Mic ou similar, incluindo **WAV 32-bit float**), sincronizá-la automaticamente ao vídeo por correlação entre a vibração das hélices captada no áudio e a vibração lida no giroscópio, permitir ajuste manual visual, e **exportar o vídeo com essa trilha embutida**, respeitando cortes.

**Decisões de produto já tomadas (não reabrir sem me perguntar):**

1. O vídeo exportado deve conter o áudio externo **sincronizado e embutido** como trilha de áudio no arquivo de saída.
2. Deve haver um **slider manual** que exibe o delay calculado e permite ajuste fino pelo usuário (mostrando o valor em **frames e em ms**), além de campo numérico editável e botão de recalcular auto-sync com **score de confiança** visível.
3. A **forma de onda (waveform)** do áudio deve ser exibida na timeline, junto aos dados do gyro, para ajuste visual.
4. Quando houver **corte (trim)** no vídeo, o áudio sincronizado deve ser cortado no ponto correspondente, com precisão de sample.
5. A **flag de exportar áudio já existente** na UI é o gate mestre. Se ela estiver ligada e houver áudio externo, o **áudio externo tem prioridade** sobre o áudio original embutido do clipe (ex.: GoPro/DJI Action que gravam áudio junto). Sem áudio externo, mantém o comportamento atual.
6. **Feature de áudio para o sync = Opção B:** usar a **banda de passagem das pás (BPF)** via STFT, não RMS bruto. Detalhado na Fase 5.
7. **Preservação do formato original do áudio é obrigatória e é o comportamento padrão.** O áudio embutido no arquivo final deve manter o formato de origem — bit depth, **32-bit float** (`pcm_f32le`) quando for o caso, sample rate original e contagem de canais original. **Nenhuma conversão silenciosa com perda** (ex.: float→AAC) é permitida. Isso tem consequências de container/codec detalhadas na Fase 3 e deve ser tratado como restrição de projeto, não como preferência.

**Distinção crítica que atravessa todas as fases — áudio de análise ≠ áudio de export:**
- O **áudio de análise** (usado só para calcular o sync na Fase 5) é downmixado para **mono f32** — descartável, existe apenas para a correlação.
- O **áudio de export** é o áudio **original preservado** (estéreo/multicanal, sample rate original, bit depth original, float se for float). O downmix mono **nunca** deve vazar para a trilha exportada.
- Mantenha, portanto, o áudio decodificado em `f32` em memória para o pipeline (trim, silêncio, offset todos operam em `f32` sem quantizar para inteiro em nenhum estágio intermediário), preservando os metadados de formato original (canais, sample rate, bit depth de origem) para reencodar corretamente no fim.

**Regras de trabalho:**

- Trabalhe em **fatias verticais testáveis**, na ordem das fases abaixo. Não pule etapas. Ao fim de cada fase, o projeto deve **compilar** e você deve me dar instruções de como validar manualmente.
- Isole toda a lógica nova em `src/core/audio/` (módulo novo). Minimize mudanças em código existente; quando precisar tocar, explique o porquê.
- O estado do áudio (caminho do arquivo, offset, sample_rate, parâmetros de banda) deve ser **serializado no arquivo de projeto `.gyroflow`** e sobreviver entre sessões.
- Cada fase = um ou mais commits coerentes com mensagem clara. Rode `cargo fmt` e `cargo clippy` antes de considerar uma fase pronta.
- **Nunca invente APIs.** Antes de usar qualquer função do core, do `ffmpeg-next` fork ou da timeline QML, **leia o código real** e cite o arquivo/linha. Se não encontrar, me diga e proponha alternativas — não improvise assinatura de função.
- Comente em português; nomes de identificadores em inglês (consistência com o upstream).

---

## FASE 0 — Reconhecimento e setup (nenhuma feature ainda)

Antes de escrever qualquer lógica, produza um **relatório de reconhecimento** (`docs/audio-sync/recon.md`) respondendo, com referências a arquivos e linhas reais do repositório:

1. **Build:** confirme o toolchain necessário lendo `BUILD.md` / scripts de build. Liste os passos exatos para compilar neste ambiente. Faça um build limpo do projeto **sem alterações** e confirme que passa.
2. **Estrutura do core:** onde vivem `GyroSource` / o parsing do gyro? Qual a taxa de amostragem típica e o formato (`[f32;3]`? struct?) das amostras de velocidade angular? Como acessá-las programaticamente?
3. **Render/export:** localize onde os streams de saída são montados no export (`src/core/rendering.rs` e o wrapper ffmpeg). **Responda especificamente:** (a) a API do fork `ffmpeg-next` permite **injetar um stream de áudio PCM próprio** na saída, ou vamos precisar modificar o fork? Cite as funções envolvidas. (b) Quais **codecs de áudio e containers** o pipeline de export suporta hoje? Especificamente: dá para escrever **`pcm_f32le` (32-bit float)** na saída, e em quais containers (MOV? MKV? MP4)? Como o container de saída é escolhido atualmente (segue a extensão? é fixo?)? Isso decide como implementaremos a preservação de formato float na Fase 3.
4. **Flag de áudio:** encontre a flag de "exportar áudio" existente na UI e onde ela é consumida no export. Como o áudio original é passado hoje (copy/remux)?
5. **Trim ranges:** como o corte é modelado no core? É um único par in/out, ou múltiplos segmentos/ranges? Cite a estrutura de dados.
6. **Timeline QML:** localize o componente que desenha os dados do gyro na timeline. Como ele recebe dados do Rust? Como o zoom é representado? Onde eu adicionaria uma nova "lane" para a waveform?
7. **Projeto `.gyroflow`:** onde está o serializador do arquivo de projeto? Como adicionar campos novos de forma retrocompatível?

**Entregável da fase:** o `recon.md` preenchido + build limpo passando. **Pare e me mostre o relatório antes de prosseguir.** As fases seguintes podem precisar de ajuste conforme o que você descobrir aqui (especialmente itens 3 e 5).

---

## FASE 1 — Decode de áudio + waveform na timeline (offset fixo = 0)

Objetivo: importar um áudio e vê-lo desenhado na timeline. Sem sync ainda.

Crie o esqueleto do módulo:

```
src/core/audio/
  mod.rs         // struct AudioTrack + API pública do módulo
  decode.rs      // arquivo → Vec<f32> mono + sample_rate
  waveform.rs    // geração de peak buckets (min/max) por nível de zoom
```

**`decode.rs`:**
- Decodifique o áudio preservando os **metadados de formato original**: bit depth de origem, se é **float ou int**, sample rate original e **número de canais** original. Guarde isso na `AudioTrack` — será necessário para reencodar sem perda na Fase 3.
- Mantenha as amostras internamente em `f32` (preserva precisão de float e é conveniente para o processamento), mas **registre o formato de origem** (ex.: um enum `SourceFormat { F32, S16, S24, S32, ... }` + `channels` + `sample_rate`). Isso permite reconstruir a saída no formato certo depois.
- **Preserve os canais originais** para o export (estéreo/multicanal). Produza uma versão **mono downmix separada, marcada como uso exclusivo de análise/sync** — nunca reaproveite o mono para exportar.
- **Suporte obrigatório a WAV 32-bit float** (IEEE float, format tag 3). Use `symphonia` ou `hound` para WAV isolado; reaproveite o ffmpeg do projeto se for mais coerente com o resto. Justifique a escolha no código.
- Trate erros de arquivo inválido/corrompido retornando `Result` com mensagem útil para a UI.

**`waveform.rs`:**
- **Não** desenhe sample-a-sample na UI. Gere no Rust um array de **peak buckets** (par min/max por bucket de pixel) na resolução do zoom atual.
- Exponha função tipo `fn peaks(&self, samples_per_bucket: usize) -> Vec<(f32, f32)>`. Recompute quando o zoom mudar.

**`mod.rs`:**
```rust
pub enum SourceFormat { F32, S32, S24, S16, U8 /* ... conforme necessário */ }

pub struct AudioTrack {
    pub path: String,

    // ---- Dados para EXPORT (formato original preservado) ----
    pub samples: Vec<f32>,       // TODOS os canais, intercalados ou por canal (documente); f32 em memória
    pub channels: u16,           // canais originais (NÃO mono)
    pub sample_rate: u32,        // sample rate original
    pub source_format: SourceFormat, // p/ reencodar sem perda (float continua float)

    // ---- Dados para ANÁLISE/SYNC (descartável) ----
    pub mono_analysis: Vec<f32>, // downmix mono só p/ correlação — nunca vai p/ o export

    pub offset_seconds: f64,     // 0.0 nesta fase
    // parâmetros de sync entram na Fase 5
}
```
> A separação entre `samples`/`channels`/`source_format` (export) e `mono_analysis` (sync) é o que garante a preservação do formato original ponta a ponta.

**UI (QML):**
- Botão "Import external audio" no painel apropriado.
- Nova lane na timeline desenhando a waveform via `Canvas` ou item customizado, consumindo os peak buckets do Rust.
- Exponha a `AudioTrack` ao QML via `qmetaobject-rs` (siga o padrão já usado para o gyro — leia o código existente antes).

**Validação:** importo um WAV (incluindo um 32-bit float, estéreo) e vejo a waveform alinhada ao início da timeline, redesenhando ao dar zoom. Confirmo (via log/debug) que a `AudioTrack` registrou corretamente **canais, sample rate e formato de origem (float)** — e que existe um `mono_analysis` separado marcado como uso de análise.

---

## FASE 2 — Slider manual + persistência no projeto

Objetivo: mover a waveform manualmente e salvar isso no `.gyroflow`.

- Adicione `offset_seconds` como valor editável via **slider** + **campo numérico**. Exiba simultaneamente em **frames** (usando o fps do projeto) e em **ms**.
- Arrastar o slider **desloca visualmente a waveform em tempo real** (apenas re-mapeia a posição de desenho; não recomputa nada do áudio).
- **Serialize** `path`, `sample_rate`, `offset_seconds` e a **preferência de preservação de formato** (`preserve_original_format`, default `true`) no arquivo de projeto. Ao reabrir, o áudio recarrega do `path` (redetectando canais/bit depth/float da origem) e o offset ajustado é restaurado. Faça isso de forma retrocompatível (projetos antigos sem esses campos devem abrir normalmente).

**Validação:** ajusto o offset, salvo, fecho, reabro — a waveform volta na posição ajustada.

---

## FASE 3 — Export com muxing + prioridade + respeito à flag

Objetivo: exportar o vídeo com o áudio externo embutido, no offset atual (ainda manual).

Crie `src/core/audio/export.rs`.

**Lógica de seleção de fonte (gate = flag existente):**
```
se (flag_exportar_audio ligada):
    se (existe audio_externo):      usar audio_externo (com offset aplicado)
    senão se (existe audio_embutido): comportamento atual (copy/remux)
    senão:                           sem áudio
senão:
    sem áudio
```
Não crie flag nova; a flag existente continua sendo o interruptor mestre.

**Muxing (Rota B — buffer próprio) com PRESERVAÇÃO DE FORMATO obrigatória:**

- Gere você mesmo o buffer de áudio final já alinhado ao offset, **preservando canais e sample rate originais**, e o encode na saída **mantendo o formato de origem**:
  - Se a origem é **32-bit float → escreva `pcm_f32le`**. Nunca converta float para AAC ou para PCM inteiro por padrão.
  - Se a origem é PCM inteiro (16/24/32-bit), escreva o PCM equivalente (ou mantenha, no mínimo, sem downgrade de bit depth).
- **Sem quantização intermediária:** o pipeline (offset, silêncio, na Fase 4 o trim) opera em `f32`; a conversão para o formato de saída acontece **só no encode final**, uma única vez.

**Política de container/codec (regra de decisão, não conversão silenciosa):**
```
detectar formato de origem (float?, bit depth, sample rate, canais)

se preservar_formato (DEFAULT = true):
    exigir codec/container de saída que suporte o formato exato
    (ex.: 32-bit float => pcm_f32le em MOV ou MKV)

    se o container de saída escolhido para o VÍDEO não suportar esse áudio
    (ex.: float em MP4):
        NÃO converter em silêncio. Em vez disso:
          - avisar o usuário claramente, e
          - oferecer: (a) trocar o container de saída (MOV/MKV), ou
                      (b) aceitar downgrade explícito (converter), com aviso visível
senão (usuário optou explicitamente por não preservar):
    usar o codec padrão do container escolhido
```
- Exponha na UI o estado de preservação (ex.: um selo/texto indicando "áudio: 32-bit float preservado" vs "áudio será convertido"). Qualquer perda de precisão tem de ser **visível antes do export**, nunca uma surpresa no arquivo final.

- **Offset:** o mapeamento é `t_audio = t_video + offset`. Nesta fase (sem corte), alinhe o áudio ao início do vídeo conforme o offset, preenchendo com **silêncio** (no mesmo formato/canais) onde o áudio não cobre o vídeo e cortando o excedente, de modo que **áudio e vídeo tenham a mesma duração** de saída.

- **Dependência da Fase 0:** se o recon revelou que o fork `ffmpeg-next` **não** escreve `pcm_f32le` facilmente ou não deixa injetar o stream, apresente o plano de modificação do fork **antes** de implementar e me avise do aumento de escopo. A preservação de float é requisito — se ela exigir mexer no fork, mexemos.

**Validação:**
1. Exporto com a flag ligada e um áudio de entrada **32-bit float**; inspeciono o arquivo final (ex.: `ffprobe`) e confirmo que a trilha saiu como **`pcm_f32le`, com os canais e sample rate originais**, e no offset correto.
2. Confirmo que o áudio externo substituiu o embutido, e que com a flag desligada sai sem áudio.
3. Testo o caso de conflito (float + container que não suporta): confirmo que **recebo o aviso e a escolha**, e que não houve conversão silenciosa.

---

## FASE 4 — Cortes (trim) respeitando o áudio

Objetivo: cortar o vídeo e ter o áudio cortado no ponto correspondente, com precisão de sample.

- Para um corte em `[t_in, t_out]` do **vídeo**, corte o áudio em `[t_in + offset, t_out + offset]`.
- **Trabalhe sempre em samples inteiros:** `idx = round((t_in + offset) * sample_rate)`. Cuidado com a cadeia frame→tempo→sample para não acumular erro de meio-frame.
- Se o corte começar antes do início do áudio (offset muito negativo) ou terminar depois do fim, **preencha com silêncio** (mesmo número de canais e formato) — nunca entregue um buffer mais curto que o trecho de vídeo, ou o mux desalinha.
- O corte opera sobre o **áudio de export multicanal preservado**, não sobre o mono de análise. O formato de origem (float etc.) continua intacto até o encode final.
- Se a Fase 0 revelou **múltiplos segmentos/ranges** no export, aplique o mapeamento por segmento e concatene os buffers de áudio na mesma ordem.

**Validação:** defino trim in/out (e múltiplos ranges, se suportado), exporto, e o áudio bate frame a frame com o vídeo cortado, sem drift perceptível.

---

## FASE 5 — Auto-sync por correlação (Opção B: banda das pás)

Objetivo: calcular automaticamente o offset inicial. Este é o último por depender mais de material real.

Crie:
```
src/core/audio/
  features.rs    // envelopes de vibração (áudio via STFT/BPF, gyro via passa-alta)
  sync.rs        // cross-correlation por FFT → (offset_seconds, confidence)
```

**`features.rs` — lado do áudio (Opção B):**
- Opere sobre o **`mono_analysis`** (downmix mono de análise), **não** sobre o áudio de export. O resultado do sync (um offset temporal) é depois aplicado ao áudio de export multicanal preservado.
- STFT com janela de Hann (~2048 amostras, hop ~512), produzindo envelope a `sample_rate/hop` Hz.
- Para cada frame do STFT, integre a energia na **banda de passagem das pás**. Suporte:
  - **banda fixa** configurável (default 150–900 Hz, cobre BPF+harmônicos da maioria dos drones), e
  - **auto-band:** média do espectro no tempo, ignora <80 Hz (vento/rumble), acha o pico dominante e integra energia em torno dele (~±40%).
- Aplique **log-compressão** (`ln(energia + 1e-9)`) no envelope.
- **Não** assuma que o RPM é conhecido — deixe o sinal revelar a banda.

**`features.rs` — lado do gyro:**
- Magnitude `sqrt(gx²+gy²+gz²)` → **passa-alta** (~30 Hz, remove movimento intencional/DC) → energia por janela, **reamostrada para a mesma taxa** do envelope de áudio → log-compressão.
- **Atenção ao Nyquist do gyro:** se o gyro amostra a `R` Hz, ele só enxerga vibração até `R/2`; acima disso a BPF aparece como aliasing. Por isso correlacionamos **envelopes de energia** (robustos a isso), não espectros diretos. Documente isso no código.

**`sync.rs`:**
- Normalize ambos os envelopes (subtrai média, divide pelo desvio).
- **Cross-correlation via FFT** (`rustfft`) — o pico dá o offset em amostras do envelope → converte para segundos.
- Retorne também o **valor de correlação normalizado no pico como score de confiança** (0..1), para a UI avisar quando o casamento é fraco.
- Assinatura sugerida:
  ```rust
  pub fn cross_correlate(
      audio_env: &[f32],
      gyro_env: &[f32],
      env_rate_hz: f32,
  ) -> SyncResult; // { offset_seconds: f64, confidence: f32 }
  ```

**Parâmetros expostos no `.gyroflow`:** `band_lo_hz`, `band_hi_hz`, `auto_band`, `highpass_hz`, com defaults sensatos. Assim drones com BPF incomum ou gyro de taxa baixa se ajustam sem recompilar.

**UI:**
- Botão "Auto-sync" que roda a correlação e preenche o slider da Fase 2.
- Exibir o **score de confiança** ao lado do resultado.
- O usuário ainda pode ajustar manualmente por cima (o auto só define o valor inicial).

**Validação (importante):** teste com um clipe de **offset conhecido** — grave uma palma/claquete na frente do drone com o mic ligado. Compare o offset detectado com o real. Se o auto-band pegar vento em vez das pás, caia para a banda fixa como fallback. Reporte o erro medido (em ms) no seu relato da fase.

---

## Estrutura final esperada do módulo

```
src/core/audio/
  mod.rs         // AudioTrack + API pública
  decode.rs      // decode → Vec<f32> mono + sample_rate (inclui WAV 32-bit float)
  waveform.rs    // peak buckets por zoom
  features.rs    // envelopes de vibração (áudio BPF/STFT + gyro passa-alta)
  sync.rs        // cross-correlation FFT → (offset, confidence)
  export.rs      // trim + silêncio + montagem do stream de saída
docs/audio-sync/
  recon.md       // relatório da Fase 0
```

Mantenha a lógica de sync **testável sem UI**: entradas `(gyro_env, audio_samples, sample_rate)` → saída `(offset_seconds, confidence)`. Adicione testes unitários com sinais sintéticos (dois envelopes com offset conhecido) validando que a correlação recupera o offset.

---

## Ao final de cada fase, me entregue

1. O que foi feito e quais arquivos mudaram (com o porquê das mudanças em código existente).
2. Confirmação de que `cargo build`, `cargo fmt` e `cargo clippy` passam.
3. **Instruções passo a passo de validação manual** que eu executo.
4. Riscos/dúvidas em aberto e o que a próxima fase pressupõe.

**Comece agora pela Fase 0. Não avance para a Fase 1 sem me mostrar o `recon.md` e receber meu OK.**

---

## Notas de risco a manter em mente (não são tarefas, são vigilância)

- **Deriva de clock:** gravador externo e câmera têm clocks independentes; em clipes longos o offset "escorrega". Um único offset pode não bastar. Se aparecer drift nos testes, me avise — podemos avaliar estimar também um leve fator de resample.
- **Gimbal mecânico:** se o sensor do gyro estiver isolado por gimbal, a vibração das pás chega atenuada; teste com material real cedo (isso é parte da validação da Fase 5).
- **Preservação de formato vs container do vídeo:** o requisito de manter 32-bit float pode colidir com o container que o usuário escolheu para o vídeo (float não cabe bem em MP4). A resolução está na Fase 3 (avisar + oferecer troca de container ou downgrade explícito), mas vigie isso: o pior resultado possível é um arquivo final com áudio silenciosamente rebaixado. Se em algum ponto você se pegar convertendo float sem avisar o usuário, pare — isso viola a decisão 7.
- **Upstream:** considere abrir uma issue/discussion no repo original descrevendo a feature antes de investir muito, para saber onde encaixar e se há interesse em merge. Não bloqueia o trabalho no fork. Guarde nos planos do projeto para seguir.

---

## Anexo — Desvios e decisões tomadas durante a execução

Registrados aqui para manter a especificação como fonte única. Cada item cita a fase afetada.

- **`export.rs` fica em `src/rendering/audio_export.rs`, não em `src/core/audio/export.rs`** (Fase 3).
  Razão: `gyroflow-core` não tem `ffmpeg-next` entre suas dependências e a "Estrutura final esperada"
  colocaria encode/mux dentro dele. A lógica pura (montagem de buffer, offset, trim, silêncio)
  permanece em `src/core/audio/export.rs` e continua testável sem ffmpeg; apenas o encode/mux
  vive no crate raiz. Decidido em 2026-08-19.
- **Container sugerido no conflito float×MP4 = `.mov`** (Fase 3). `.mov` já é o container de
  ProRes/DNxHD/CineForm no `Export.qml`, e `App.qml:729` já traz a mensagem oficial recomendando
  `.mov` nessa situação. MKV continua funcionando no pipeline (`ffmpeg_processor.rs:277`) para quem
  digitar a extensão, mas não será exposto na UI. Decidido em 2026-08-19.
- **`IMUData` usa `[f64;3]`, não `[f32;3]`** (Fase 5). Verificado em `telemetry-parser` rev
  `77a3b81`, `src/util.rs:289-294`. O áudio chega em `f32`: a conversão precisa ser explícita.
- **A taxa de amostragem do gyro não precisa ser estimada para alinhar séries** (Fases 1 e 5).
  `optimsync.rs:34-51` já interpola o gyro linearmente por timestamp via `partition_point`. A grade
  temporal vem do áudio. A estimativa de taxa continua necessária apenas para o critério de Nyquist
  documentado na Fase 5.
