# 🌐 FASE 3: WebSocket Client + Gemini API

## 📋 Objetivo da Fase

Implementar cliente WebSocket completo com suporte a TLS/WSS e integração com a API Gemini para comunicação em tempo real, preparando o terreno para streaming de áudio na Fase 4.

---

## ✅ Pré-requisitos

Antes de começar esta fase, certifica-te que:

- ✅ Completaste a **Fase 2** (TLS/SSL funcional)
- ✅ O EVA Daemon compila sem erros
- ✅ Tens a `GOOGLE_API_KEY` configurada
- ✅ Conexões TLS funcionam corretamente

---

## 🎯 Passos da Implementação

### Passo 3.1: Criar Módulo WebSocket

Criamos um cliente WebSocket que suporta conexões seguras (WSS) automaticamente.

**Arquivo:** [`src/websocket.rs`](file:///d:/dev/Redox-EVA/eva-daemon/src/websocket.rs)

**Funcionalidades:**
- ✅ Conexão automática WSS (TLS)
- ✅ Envio de mensagens texto
- ✅ Envio de mensagens binárias (para áudio PCM)
- ✅ Recebimento de mensagens
- ✅ Ping/Pong para manter conexão ativa
- ✅ Fechamento gracioso da conexão

**Código principal:**
```rust
pub struct WebSocketClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocketClient {
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>>
    pub async fn send_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>>
    pub async fn send_binary(&mut self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>>
    pub async fn receive(&mut self) -> Result<Option<Message>, Box<dyn std::error::Error>>
    pub async fn ping(&mut self) -> Result<(), Box<dyn std::error::Error>>
    pub async fn close(mut self) -> Result<(), Box<dyn std::error::Error>>
}
```

---

### Passo 3.2: Criar Módulo Gemini API

Implementamos um cliente para a API Gemini com suporte a WebSocket nativo.

**Arquivo:** [`src/gemini.rs`](file:///d:/dev/Redox-EVA/eva-daemon/src/gemini.rs)

**Funcionalidades:**
- ✅ Conexão via WebSocket ao Gemini
- ✅ Configuração de modelo e voz
- ✅ Envio de áudio PCM (Base64)
- ✅ Envio de texto
- ✅ Recebimento de respostas (texto + áudio)

**Estrutura de configuração:**
```rust
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
    pub ws_url: String,
}
```

**Protocolo Gemini:**
1. **Setup** - Configurar modelo e parâmetros
2. **Realtime Input** - Enviar áudio/texto
3. **Server Content** - Receber respostas

---

### Passo 3.3: Atualizar Main para Testes

Atualizamos o `main.rs` para testar todas as funcionalidades.

**Testes implementados:**
1. **WebSocket Echo** - Servidor público de teste
2. **EVA Mind Backend** - Conexão com backend Go
3. **Gemini API** - Integração completa

---

### Passo 3.4: Atualizar Cargo.toml

```toml
[package]
name = "eva-daemon"
version = "0.3.0"  # ← Atualizado para Fase 3
edition = "2021"
```

---

## 🧪 Testes

### Teste 1: WebSocket Echo

```bash
cd d:\dev\Redox-EVA\eva-daemon
cargo run --release
```

**Saída esperada:**
```
[1/3] Testando WebSocket básico...
✅ Conectado ao servidor echo
📤 Mensagem enviada
📥 Resposta recebida: Text("Hello from EVA Daemon!")
✅ Teste WebSocket básico completo
```

### Teste 2: EVA Mind Backend

```bash
# Backend deve estar rodando em wss://eva-ia.org:8090/ws/pcm
cargo run --release
```

**Saída esperada:**
```
[2/3] Testando conexão com EVA Mind backend...
✅ Conectado ao EVA Mind backend!
📤 Ping enviado ao backend
✅ Teste EVA Mind backend completo
```

### Teste 3: Gemini API

```bash
# Configurar API key
$env:GOOGLE_API_KEY="sua_chave_aqui"
cargo run --release
```

**Saída esperada:**
```
[3/3] Testando conexão com Gemini API...
✅ Conectado ao Gemini!
✅ Setup enviado ao Gemini
📤 Mensagem enviada ao Gemini
🤖 Gemini: [resposta do modelo]
✅ Teste Gemini completo
```

---

## 🔧 Configuração

### Variáveis de Ambiente

```bash
# Windows PowerShell
$env:GOOGLE_API_KEY="AIzaSyAJq7G4wg_7GSlz1CmgKxqCtLlkzQ3YmTQ"

# Linux/macOS
export GOOGLE_API_KEY="AIzaSyAJq7G4wg_7GSlz1CmgKxqCtLlkzQ3YmTQ"
```

### Endpoints

| Serviço | URL | Descrição |
|---------|-----|-----------|
| Echo Test | `wss://echo.websocket.org/` | Servidor de teste público |
| EVA Mind | `wss://eva-ia.org:8090/ws/pcm` | Backend Go do EVA |
| Gemini API | `wss://generativelanguage.googleapis.com/ws/...` | API Gemini WebSocket |

---

## 🐛 Troubleshooting

### Erro: "Connection refused"

**Causa:** Backend não está rodando ou URL incorreta

**Solução:**
```bash
# Verificar se o backend está ativo
curl -I https://eva-ia.org:8090/health
```

### Erro: "Invalid API key"

**Causa:** `GOOGLE_API_KEY` não configurada ou inválida

**Solução:**
```bash
# Verificar se a variável está definida
echo $env:GOOGLE_API_KEY  # Windows
echo $GOOGLE_API_KEY      # Linux
```

### Erro: "TLS handshake failed"

**Causa:** Certificados SSL inválidos

**Solução:**
- Verificar data/hora do sistema
- Atualizar certificados CA do sistema

---

## 📊 Checklist da Fase 3

- [x] Criar `src/websocket.rs`
- [x] Implementar conexão WSS
- [x] Implementar envio/recebimento de mensagens
- [x] Criar `src/gemini.rs`
- [x] Implementar protocolo Gemini
- [x] Atualizar `main.rs` com testes
- [x] Atualizar `Cargo.toml` para v0.3.0
- [x] Compilar sem erros
- [x] Testar WebSocket echo
- [x] Testar conexão Gemini
- [x] Documentar em `fase3.md`

---

## 🎓 Conceitos Aprendidos

### WebSocket vs HTTP

| Característica | HTTP | WebSocket |
|----------------|------|-----------|
| Conexão | Request/Response | Bidirecional persistente |
| Overhead | Alto (headers repetidos) | Baixo (conexão única) |
| Latência | Alta | Baixa |
| Uso | APIs REST | Streaming, chat, tempo real |

### Protocolo WebSocket

```
Cliente                          Servidor
   |                                |
   |-------- HTTP Upgrade --------->|
   |<------- 101 Switching ---------|
   |                                |
   |===== WebSocket Frames ========>|
   |<====== WebSocket Frames =======|
   |                                |
   |-------- Close Frame ---------->|
   |<------- Close Frame -----------|
```

### Gemini WebSocket Protocol

1. **Setup Message** - Configurar modelo e parâmetros
```json
{
  "setup": {
    "model": "gemini-2.0-flash-exp",
    "generation_config": {
      "response_modalities": ["AUDIO"]
    }
  }
}
```

2. **Realtime Input** - Enviar dados
```json
{
  "realtime_input": {
    "media_chunks": [{
      "mime_type": "audio/pcm",
      "data": "<base64>"
    }]
  }
}
```

3. **Server Content** - Receber respostas
```json
{
  "serverContent": {
    "modelTurn": {
      "parts": [
        {"text": "resposta"},
        {"inlineData": {"mimeType": "audio/pcm", "data": "<base64>"}}
      ]
    }
  }
}
```

---

## 🚀 Próximos Passos

Com WebSocket e Gemini funcionando, estás pronto para a **Fase 4: Integração de Áudio**.

Na próxima fase vais:
- Implementar captura de áudio do microfone
- Criar ring buffer para streaming
- Implementar Voice Activity Detection (VAD)
- Integrar com o esquema `audio:` do Redox OS
- Conectar tudo para conversação em tempo real

---

## 📚 Recursos Adicionais

- [RFC 6455 - WebSocket Protocol](https://datatracker.ietf.org/doc/html/rfc6455)
- [Gemini API Documentation](https://ai.google.dev/gemini-api/docs)
- [tokio-tungstenite](https://docs.rs/tokio-tungstenite/)
- [WebSocket.org Echo Test](https://www.websocket.org/echo.html)

---

**Status:** ✅ Fase 3 Completa  
**Próxima:** 🎤 Fase 4 - Integração de Áudio  
**Versão EVA:** 0.3.0
