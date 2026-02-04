# 🚀 ROTEIRO COMPLETO: Do Zero ao Redox-EVA Funcionando

## 📋 PRÉ-REQUISITOS DO TEU SISTEMA

Antes de começar, certifica-te que tens instalado:

```bash
# Ubuntu/Debian
sudo apt install -y build-essential curl git qemu-system-x86 \
    qemu-utils libfuse-dev pkg-config libc6-dev-i386 \
    nasm make mtools

# Fedora/RHEL
sudo dnf install -y gcc gcc-c++ curl git qemu qemu-img \
    fuse-devel pkg-config glibc-devel.i686 nasm make mtools

# Arch Linux
sudo pacman -S base-devel curl git qemu qemu-arch-extra \
    fuse2 pkg-config nasm make mtools
```

**RAM recomendada:** 8GB mínimo (16GB ideal)  
**Espaço em disco:** 20GB livres  
**Tempo estimado:** 2-4 horas (primeira compilação é lenta)

---

## 🎯 PASSO 1: INSTALAR O RUST (Nightly)

O Redox precisa de Rust nightly com componentes específicos:

```bash
# Instalar rustup (se ainda não tiveres)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Instalar Rust nightly e componentes necessários
rustup default nightly
rustup component add rust-src
rustup target add x86_64-unknown-redox

# Verificar instalação
rustc --version  # Deve mostrar "nightly"
```

---

## 🗂️ PASSO 2: CLONAR O REDOX OS

```bash
# Criar diretório de trabalho
mkdir -p ~/redox-dev
cd ~/redox-dev

# Clonar o repositório principal do Redox
git clone https://gitlab.redox-os.org/redox-os/redox.git
cd redox

# Este comando baixa TODOS os submódulos (pode demorar 15-30 min)
git submodule update --init --recursive
```

**O que acabaste de baixar:**
- `kernel/` - O microkernel do Redox
- `relibc/` - A biblioteca C padrão em Rust
- `drivers/` - Drivers de hardware (áudio, rede, etc.)
- `userutils/` - Utilitários de sistema (aqui vais meter o EVA)

---

## ⚙️ PASSO 3: CONFIGURAR O AMBIENTE DE BUILD

```bash
# Ainda dentro de ~/redox-dev/redox/

# Instalar ferramentas de build do Redox
make prefix

# Configurar para desktop (tem drivers de áudio e rede)
make config recipe=desktop

# IMPORTANTE: Editar configuração para habilitar áudio no QEMU
nano .config
```

**Adiciona estas linhas no `.config`:**
```makefile
# Habilitar áudio no QEMU
QEMU_FLAGS += -device intel-hda -device hda-duplex
QEMU_FLAGS += -audiodev pa,id=snd0

# Habilitar rede (para conectar ao Gemini)
QEMU_FLAGS += -netdev user,id=net0 -device e1000,netdev=net0
```

Salva com `Ctrl+O`, sai com `Ctrl+X`.

---

## 🧪 PASSO 4: TESTAR A BUILD BÁSICA (Sem EVA ainda)

```bash
# Primeira compilação (DEMORA! Pode levar 1-2 horas)
make all

# Se tudo correr bem, iniciar o Redox no QEMU
make qemu

# Deves ver o Redox OS a arrancar
# Podes fechar com Ctrl+A depois X
```

**Se der erro:**
- `error: linker 'cc' not found` → Falta gcc: `sudo apt install build-essential`
- `FUSE error` → Falta biblioteca: `sudo apt install libfuse-dev`
- `nasm not found` → Instala: `sudo apt install nasm`

---

## 🧠 PASSO 5: CRIAR O PROJETO EVA DAEMON

Agora vamos adicionar o nosso código ao Redox:

```bash
cd ~/redox-dev/redox/cookbook/recipes/

# Criar a receita (recipe) do EVA
mkdir -p eva-daemon
cd eva-daemon

# Criar o arquivo de configuração da receita
nano recipe.toml
```

**Conteúdo do `recipe.toml`:**
```toml
[source]
git = "https://github.com/teu-username/eva-daemon.git"  # Vais criar este repo
branch = "main"

[build]
template = "cargo"

# Dependências do sistema
dependencies = [
    "audio",
    "network"
]
```

---

## 📝 PASSO 6: CRIAR O REPOSITÓRIO GIT DO EVA

Abre outro terminal (mantém o do Redox aberto):

```bash
# Criar diretório para o código do EVA
mkdir -p ~/redox-dev/eva-daemon
cd ~/redox-dev/eva-daemon

# Inicializar git
git init

# Criar estrutura básica
mkdir -p src tests

# Criar Cargo.toml
nano Cargo.toml
```

**Conteúdo do `Cargo.toml`:**
```toml
[package]
name = "eva-daemon"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.40", features = ["rt-multi-thread", "net", "io-util", "time"], default-features = false }
tokio-tungstenite = { version = "0.20", features = ["rustls-tls-webpki-roots"] }
futures-util = "0.3"
url = "2.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.22"

# Específico do Redox
[target.'cfg(target_os = "redox")'.dependencies]
redox_syscall = "0.5"
```

---

## 💻 PASSO 7: CÓDIGO INICIAL DO EVA (Teste de Rede)

Vamos começar com um teste simples (antes do áudio):

```bash
# Ainda em ~/redox-dev/eva-daemon/
nano src/main.rs
```

**Conteúdo do `src/main.rs` (versão teste):**
```rust
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn main() {
    println!("🧠 EVA Daemon v0.1.0 - Teste de Conectividade");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Teste 1: DNS básico
    println!("\n[1/3] Testando resolução DNS...");
    match std::net::ToSocketAddrs::to_socket_addrs(&"google.com:443") {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                println!("✅ DNS OK: google.com → {}", addr);
            }
        }
        Err(e) => {
            eprintln!("❌ Falha DNS: {}", e);
            return;
        }
    }
    
    // Teste 2: Conexão TCP básica
    println!("\n[2/3] Testando conexão TCP...");
    match TcpStream::connect_timeout(
        &"google.com:443".parse().unwrap(),
        Duration::from_secs(10)
    ) {
        Ok(stream) => {
            println!("✅ TCP OK: Conectado a google.com:443");
            println!("   Peer: {:?}", stream.peer_addr());
        }
        Err(e) => {
            eprintln!("❌ Falha TCP: {}", e);
            return;
        }
    }
    
    // Teste 3: TLS (vai falhar agora, mas mostra o erro)
    println!("\n[3/3] Testando TLS/SSL...");
    println!("⚠️  TLS ainda não implementado nesta versão");
    println!("    Próximo passo: adicionar rustls");
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Teste de conectividade concluído!");
}
```

---

## 🔨 PASSO 8: COMPILAR O EVA PARA REDOX

```bash
# Ainda em ~/redox-dev/eva-daemon/

# Compilar para o target do Redox
cargo build --target x86_64-unknown-redox --release

# Se compilar sem erros, o binário está em:
ls -lh target/x86_64-unknown-redox/release/eva-daemon
```

**Possíveis erros nesta fase:**
- `error: could not find target` → Falta: `rustup target add x86_64-unknown-redox`
- `linker error` → Pode ser normal, vamos usar o build system do Redox

---

## 🔗 PASSO 9: INTEGRAR COM O BUILD SYSTEM DO REDOX

```bash
# Voltar para o diretório do Redox
cd ~/redox-dev/redox/

# Fazer push do teu código EVA para o GitHub/GitLab primeiro
cd ~/redox-dev/eva-daemon/
git add .
git commit -m "EVA Daemon v0.1.0 - Network test"

# Criar repo no GitHub e fazer push
# git remote add origin https://github.com/teu-username/eva-daemon.git
# git push -u origin main

# Voltar para o Redox e atualizar a receita
cd ~/redox-dev/redox/cookbook/recipes/eva-daemon/
nano recipe.toml
```

**Atualiza o `recipe.toml` com o URL correto:**
```toml
[source]
git = "https://github.com/teu-username/eva-daemon.git"  # TEU URL AQUI
branch = "main"
```

---

## 🎮 PASSO 10: COMPILAR REDOX COM EVA INCLUÍDO

```bash
cd ~/redox-dev/redox/

# Adicionar EVA à configuração desktop
nano config/x86_64/desktop.toml
```

**Adiciona no final do ficheiro:**
```toml
[packages]
# ... (outros pacotes já existentes)
eva-daemon = "recipe"  # Adiciona esta linha
```

```bash
# Recompilar o sistema (mais rápido que a primeira vez)
make rebuild

# Se tudo correr bem, iniciar o Redox
make qemu
```

---

## ✅ PASSO 11: TESTAR O EVA DENTRO DO REDOX

Quando o Redox arrancar no QEMU:

```bash
# No terminal do Redox (dentro do QEMU)
eva-daemon

# Deves ver:
# 🧠 EVA Daemon v0.1.0 - Teste de Conectividade
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# [1/3] Testando resolução DNS...
# ✅ DNS OK: google.com → 142.250.xxx.xxx
# ...
```

---

## 🐛 TROUBLESHOOTING COMUM

| Erro | Solução |
|------|---------|
| `make: command not found` | `sudo apt install make` |
| `nasm: command not found` | `sudo apt install nasm` |
| `FUSE error` | `sudo apt install libfuse-dev` |
| `Network unreachable` no QEMU | Verifica se adicionaste `-netdev user` no `.config` |
| `No such file: audio:record` | Driver de áudio não carregou, reinicia o QEMU |
| Compilação trava/congela | Reduz jobs: `make -j2` (ao invés de usar todos os cores) |

# 🚀 FASES 2-5: Implementação Completa do Redox-EVA

Vou detalhar cada fase com código completo e instruções práticas.

---

## 🔐 FASE 2: Adicionar TLS/SSL com `rustls`

### Passo 2.1: Atualizar Dependências

```bash
cd ~/redox-dev/eva-daemon/
nano Cargo.toml
```

**Atualiza o `Cargo.toml`:**
```toml
[package]
name = "eva-daemon"
version = "0.2.0"
edition = "2021"

[dependencies]
tokio = { version = "1.40", features = ["rt-multi-thread", "net", "io-util", "time"], default-features = false }
tokio-tungstenite = { version = "0.20", features = ["rustls-tls-webpki-roots"] }
rustls = "0.23"
rustls-pemfile = "2.0"
webpki-roots = "0.26"
tokio-rustls = "0.26"
futures-util = "0.3"
url = "2.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.22"

[target.'cfg(target_os = "redox")'.dependencies]
redox_syscall = "0.5"
```

### Passo 2.2: Criar Módulo TLS

```bash
nano src/tls.rs
```

**Conteúdo do `src/tls.rs`:**
```rust
use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};

pub struct TlsManager {
    connector: TlsConnector,
}

impl TlsManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Carregar certificados raiz (CA certificates)
        let mut root_store = RootCertStore::empty();
        
        // Usar certificados do sistema
        for cert in rustls_native_certs::load_native_certs()? {
            root_store.add(cert).ok();
        }
        
        // Fallback: usar certificados embutidos do webpki
        root_store.extend(
            webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .cloned()
        );

        // Configurar cliente TLS
        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));

        Ok(Self { connector })
    }

    pub async fn connect(
        &self,
        domain: &str,
        port: u16,
    ) -> Result<TlsStream<TcpStream>, Box<dyn std::error::Error>> {
        // Conectar TCP primeiro
        let addr = format!("{}:{}", domain, port);
        let tcp_stream = TcpStream::connect(&addr).await?;

        // Fazer handshake TLS
        let server_name = rustls::pki_types::ServerName::try_from(domain.to_string())?;
        let tls_stream = self.connector.connect(server_name, tcp_stream).await?;

        Ok(tls_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tls_connection() {
        let tls = TlsManager::new().expect("Failed to create TLS manager");
        
        let result = tls.connect("google.com", 443).await;
        assert!(result.is_ok(), "TLS connection should succeed");
    }
}
```

### Passo 2.3: Atualizar o Main para Testar TLS

```bash
nano src/main.rs
```

**Conteúdo atualizado do `src/main.rs`:**
```rust
mod tls;

use tls::TlsManager;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 EVA Daemon v0.2.0 - Teste TLS/SSL");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Criar gerenciador TLS
    println!("\n[1/3] Inicializando TLS Manager...");
    let tls_manager = TlsManager::new()?;
    println!("✅ TLS Manager criado com sucesso");

    // Conectar ao Google via TLS
    println!("\n[2/3] Conectando a google.com:443 via TLS...");
    let mut stream = tls_manager.connect("google.com", 443).await?;
    println!("✅ Handshake TLS completo!");

    // Fazer requisição HTTP simples
    println!("\n[3/3] Enviando requisição HTTP GET...");
    let request = "GET / HTTP/1.1\r\nHost: google.com\r\nConnection: close\r\n\r\n";
    stream.write_all(request.as_bytes()).await?;

    // Ler resposta
    let mut buffer = vec![0u8; 1024];
    let n = stream.read(&mut buffer).await?;
    let response = String::from_utf8_lossy(&buffer[..n]);

    println!("📥 Resposta recebida ({} bytes):", n);
    println!("{}", response.lines().take(10).collect::<Vec<_>>().join("\n"));

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ FASE 2 COMPLETA - TLS funcional!");

    Ok(())
}
```

### Passo 2.4: Testar Localmente Primeiro

```bash
# Compilar e testar no teu Linux (antes de tentar no Redox)
cargo build --release
cargo test

# Se tudo funcionar:
./target/release/eva-daemon

# Deves ver a resposta HTTP do Google
```

**Se der erro `rustls-native-certs`:**
```bash
# Adiciona ao Cargo.toml:
rustls-native-certs = "0.7"
```

---

## 🌐 FASE 3: Implementar WebSocket Client

### Passo 3.1: Criar Módulo WebSocket

```bash
nano src/websocket.rs
```

**Conteúdo do `src/websocket.rs`:**
```rust
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use url::Url;
use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;

pub struct WebSocketClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocketClient {
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        println!("🔗 Conectando ao WebSocket: {}", url);

        // Configurar TLS para WebSocket
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        
        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = tokio_tungstenite::Connector::Rustls(Arc::new(config));

        // Conectar
        let url = Url::parse(url)?;
        let (ws_stream, response) = connect_async_tls_with_config(
            url,
            None,
            false,
            Some(connector)
        ).await?;

        println!("✅ WebSocket conectado! Status: {}", response.status());

        Ok(Self { stream: ws_stream })
    }

    pub async fn send_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.send(Message::Text(text.to_string())).await?;
        Ok(())
    }

    pub async fn send_binary(&mut self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.send(Message::Binary(data)).await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        match self.stream.next().await {
            Some(Ok(msg)) => Ok(Some(msg)),
            Some(Err(e)) => Err(Box::new(e)),
            None => Ok(None),
        }
    }

    pub async fn close(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.close(None).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_echo() {
        // Usar servidor de teste público
        let mut client = WebSocketClient::connect("wss://echo.websocket.org/")
            .await
            .expect("Failed to connect");

        client.send_text("Hello WebSocket!").await.expect("Failed to send");
        
        let response = client.receive().await.expect("Failed to receive");
        assert!(response.is_some());
    }
}
```

### Passo 3.2: Testar WebSocket

```bash
nano src/main.rs
```

**Atualiza o `main.rs` para testar WebSocket:**
```rust
mod tls;
mod websocket;

use websocket::WebSocketClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 EVA Daemon v0.3.0 - Teste WebSocket");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Testar com servidor echo público
    println!("\n[1/2] Conectando ao servidor echo...");
    let mut client = WebSocketClient::connect("wss://echo.websocket.org/").await?;
    println!("✅ WebSocket conectado!");

    // Enviar mensagem
    println!("\n[2/2] Testando envio/recebimento...");
    client.send_text("Hello from EVA Daemon!").await?;
    println!("📤 Mensagem enviada");

    // Receber resposta
    if let Some(msg) = client.receive().await? {
        println!("📥 Resposta recebida: {:?}", msg);
    }

    client.close().await?;
    println!("\n✅ FASE 3 COMPLETA - WebSocket funcional!");

    Ok(())
}
```

```bash
# Testar
cargo build --release
./target/release/eva-daemon
```

---

## 🔊 FASE 4: Integrar com `audio:` Scheme

### Passo 4.1: Criar Módulo de Áudio

```bash
nano src/audio.rs
```

**Conteúdo do `src/audio.rs`:**
```rust
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Result as IoResult};
use std::path::Path;

pub struct AudioDevice {
    input: Option<File>,   // Microfone
    output: Option<File>,  // Alto-falantes
}

impl AudioDevice {
    pub fn new() -> IoResult<Self> {
        println!("🎤 Inicializando dispositivos de áudio...");

        // No Redox, áudio é acessado via esquemas (schemes)
        let input = if Path::new("audio:record").exists() {
            Some(File::open("audio:record")?)
        } else {
            println!("⚠️  Microfone não disponível (audio:record)");
            None
        };

        let output = if Path::new("audio:play").exists() {
            Some(OpenOptions::new().write(true).open("audio:play")?)
        } else {
            println!("⚠️  Alto-falante não disponível (audio:play)");
            None
        };

        Ok(Self { input, output })
    }

    /// Ler um chunk de áudio do microfone (bloqueante)
    pub fn read_input(&mut self, buffer: &mut [u8]) -> IoResult<usize> {
        match &mut self.input {
            Some(file) => file.read(buffer),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Microfone não disponível"
            )),
        }
    }

    /// Escrever áudio para os alto-falantes
    pub fn write_output(&mut self, buffer: &[u8]) -> IoResult<usize> {
        match &mut self.output {
            Some(file) => file.write(buffer),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Alto-falante não disponível"
            )),
        }
    }

    /// Testar loopback (microfone -> alto-falante)
    pub fn test_loopback(&mut self, duration_secs: u64) -> IoResult<()> {
        println!("🔄 Iniciando teste de loopback por {} segundos...", duration_secs);
        println!("   (Fale no microfone, deves ouvir a tua voz)");

        let mut buffer = vec![0u8; 4096]; // 4KB buffer
        let start = std::time::Instant::now();

        while start.elapsed().as_secs() < duration_secs {
            match self.read_input(&mut buffer) {
                Ok(n) if n > 0 => {
                    self.write_output(&buffer[..n])?;
                }
                Ok(_) => {} // Sem dados
                Err(e) => {
                    eprintln!("❌ Erro no loopback: {}", e);
                    break;
                }
            }
        }

        println!("✅ Teste de loopback concluído");
        Ok(())
    }
}

// Estrutura para buffer circular (ring buffer)
pub struct RingBuffer {
    buffer: Vec<u8>,
    write_pos: usize,
    read_pos: usize,
    size: usize,
}

impl RingBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            buffer: vec![0u8; size],
            write_pos: 0,
            read_pos: 0,
            size,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        let available = self.available_write();
        let to_write = data.len().min(available);

        for i in 0..to_write {
            self.buffer[self.write_pos] = data[i];
            self.write_pos = (self.write_pos + 1) % self.size;
        }

        to_write
    }

    pub fn read(&mut self, data: &mut [u8]) -> usize {
        let available = self.available_read();
        let to_read = data.len().min(available);

        for i in 0..to_read {
            data[i] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.size;
        }

        to_read
    }

    fn available_write(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.size - (self.write_pos - self.read_pos) - 1
        } else {
            self.read_pos - self.write_pos - 1
        }
    }

    fn available_read(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            self.size - self.read_pos + self.write_pos
        }
    }
}
```

### Passo 4.2: Testar Áudio

```bash
nano src/main.rs
```

**Atualiza para testar áudio:**
```rust
mod tls;
mod websocket;
mod audio;

use audio::AudioDevice;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 EVA Daemon v0.4.0 - Teste de Áudio");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("\n[1/1] Inicializando dispositivos de áudio...");
    let mut audio = AudioDevice::new()?;

    // Testar loopback por 5 segundos
    audio.test_loopback(5)?;

    println!("\n✅ FASE 4 COMPLETA - Áudio funcional!");

    Ok(())
}
```

---

## 🤖 FASE 5: Conectar ao Gemini 2.5 Flash

### Passo 5.1: Criar Módulo Gemini

```bash
nano src/gemini.rs
```

**Conteúdo do `src/gemini.rs`:**
```rust
use crate::websocket::WebSocketClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            model: "models/gemini-2.0-flash-exp".to_string(),
        }
    }
}

pub struct GeminiClient {
    ws: WebSocketClient,
    config: GeminiConfig,
}

impl GeminiClient {
    pub async fn connect(config: GeminiConfig) -> Result<Self, Box<dyn std::error::Error>> {
        if config.api_key.is_empty() {
            return Err("GEMINI_API_KEY não configurada".into());
        }

        let url = format!(
            "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={}",
            config.api_key
        );

        println!("🤖 Conectando ao Gemini...");
        let ws = WebSocketClient::connect(&url).await?;

        let mut client = Self { ws, config };

        // Enviar mensagem de setup
        client.send_setup().await?;

        Ok(client)
    }

    async fn send_setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let setup = json!({
            "setup": {
                "model": self.config.model,
                "generation_config": {
                    "response_modalities": ["AUDIO"],
                    "speech_config": {
                        "voice_config": {
                            "prebuilt_voice_config": {
                                "voice_name": "Kore"
                            }
                        }
                    }
                }
            }
        });

        self.ws.send_text(&setup.to_string()).await?;
        println!("✅ Setup enviado ao Gemini");

        Ok(())
    }

    pub async fn send_audio(&mut self, pcm_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let base64_audio = BASE64.encode(pcm_data);

        let message = json!({
            "realtime_input": {
                "media_chunks": [{
                    "mime_type": "audio/pcm",
                    "data": base64_audio
                }]
            }
        });

        self.ws.send_text(&message.to_string()).await?;
        Ok(())
    }

    pub async fn send_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let message = json!({
            "client_content": {
                "turns": [{
                    "role": "user",
                    "parts": [{
                        "text": text
                    }]
                }],
                "turn_complete": true
            }
        });

        self.ws.send_text(&message.to_string()).await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Option<GeminiResponse>, Box<dyn std::error::Error>> {
        if let Some(msg) = self.ws.receive().await? {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                let response: GeminiResponse = serde_json::from_str(&text)?;
                return Ok(Some(response));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Deserialize)]
pub struct GeminiResponse {
    #[serde(rename = "serverContent")]
    pub server_content: Option<ServerContent>,
}

#[derive(Debug, Deserialize)]
pub struct ServerContent {
    #[serde(rename = "modelTurn")]
    pub model_turn: Option<ModelTurn>,
}

#[derive(Debug, Deserialize)]
pub struct ModelTurn {
    pub parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
pub struct Part {
    pub text: Option<String>,
    #[serde(rename = "inlineData")]
    pub inline_data: Option<InlineData>,
}

#[derive(Debug, Deserialize)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String, // Base64
}
```

### Passo 5.2: Implementação Final Completa

```bash
nano src/main.rs
```

**Conteúdo FINAL do `src/main.rs`:**
```rust
mod tls;
mod websocket;
mod audio;
mod gemini;

use audio::{AudioDevice, RingBuffer};
use gemini::{GeminiClient, GeminiConfig};
use tokio::time::{sleep, Duration};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 EVA Daemon v1.0.0 - COMPLETO");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Verificar API Key
    if std::env::var("GEMINI_API_KEY").is_err() {
        eprintln!("❌ GEMINI_API_KEY não configurada!");
        eprintln!("   export GEMINI_API_KEY=your_key_here");
        return Ok(());
    }

    // Inicializar componentes
    println!("\n[1/3] Inicializando áudio...");
    let mut audio = AudioDevice::new()?;
    let mut input_buffer = RingBuffer::new(48000); // 1 segundo a 48kHz
    let mut output_buffer = RingBuffer::new(48000);

    println!("\n[2/3] Conectando ao Gemini...");
    let config = GeminiConfig::default();
    let mut gemini = GeminiClient::connect(config).await?;

    println!("\n[3/3] Iniciando loop principal...");
    println!("🎤 EVA está ouvindo. Fale algo...\n");

    let mut audio_chunk = vec![0u8; 4096];
    let mut should_send = false;
    let mut silence_counter = 0;

    loop {
        // CAPTURA DE ÁUDIO
        if let Ok(n) = audio.read_input(&mut audio_chunk) {
            if n > 0 {
                // VAD simples: verificar se há volume
                let volume: i32 = audio_chunk[..n]
                    .iter()
                    .map(|&x| (x as i32 - 128).abs())
                    .sum();

                if volume > 5000 { // Threshold ajustável
                    should_send = true;
                    silence_counter = 0;
                    input_buffer.write(&audio_chunk[..n]);
                } else {
                    silence_counter += 1;
                }

                // Enviar quando tiver silêncio após fala
                if should_send && silence_counter > 50 {
                    println!("📤 Enviando áudio ao Gemini...");
                    
                    let mut buffer_to_send = vec![0u8; 48000];
                    let bytes_read = input_buffer.read(&mut buffer_to_send);
                    
                    gemini.send_audio(&buffer_to_send[..bytes_read]).await?;
                    
                    should_send = false;
                    silence_counter = 0;
                }
            }
        }

        // RECEBER RESPOSTA DO GEMINI
        if let Some(response) = gemini.receive().await? {
            if let Some(content) = response.server_content {
                if let Some(turn) = content.model_turn {
                    for part in turn.parts {
                        // Texto
                        if let Some(text) = part.text {
                            println!("🤖 EVA: {}", text);
                        }

                        // Áudio
                        if let Some(inline) = part.inline_data {
                            if inline.mime_type == "audio/pcm" {
                                println!("📥 Recebendo áudio da EVA...");
                                
                                let audio_data = BASE64.decode(&inline.data)?;
                                output_buffer.write(&audio_data);
                                
                                // Reproduzir
                                let mut playback = vec![0u8; audio_data.len()];
                                let n = output_buffer.read(&mut playback);
                                audio.write_output(&playback[..n])?;
                            }
                        }
                    }
                }
            }
        }

        sleep(Duration::from_millis(10)).await;
    }
}
```

### Passo 5.3: Configurar e Compilar

```bash
# Definir API key
export GEMINI_API_KEY="tua_chave_aqui"

# Compilar versão final
cargo build --release

# Testar no Linux primeiro
./target/release/eva-daemon

# Se funcionar, compilar para Redox
cargo build --target x86_64-unknown-redox --release
```

---

## 📦 INTEGRAÇÃO FINAL NO REDOX

```bash
cd ~/redox-dev/eva-daemon/

# Commit final
git add .
git commit -m "EVA v1.0.0 - Full integration complete"
git push

# Reconstruir Redox
cd ~/redox-dev/redox/
make rebuild
make qemu
```

**No Redox (dentro do QEMU):**
```bash
# Configurar API key
export GEMINI_API_KEY="tua_chave"

# Iniciar EVA
eva-daemon

# Fala no microfone e a EVA responde!
```
# 📁 LISTA COMPLETA DE ARQUIVOS A CRIAR/MODIFICAR

## 🆕 ARQUIVOS NOVOS A CRIAR

### No diretório `~/redox-dev/eva-daemon/` (teu projeto EVA)

```
~/redox-dev/eva-daemon/
├── Cargo.toml                    # ✏️ CRIAR - Configuração do projeto Rust
├── .gitignore                    # ✏️ CRIAR - Ignorar target/ e outros
├── README.md                     # ✏️ CRIAR - Documentação do projeto
├── LICENSE                       # ✏️ CRIAR - MIT License (opcional)
│
└── src/
    ├── main.rs                   # ✏️ CRIAR - Ponto de entrada principal
    ├── tls.rs                    # ✏️ CRIAR - Módulo de TLS/SSL
    ├── websocket.rs              # ✏️ CRIAR - Cliente WebSocket
    ├── audio.rs                  # ✏️ CRIAR - Gerenciamento de áudio
    └── gemini.rs                 # ✏️ CRIAR - Cliente API Gemini
```

---

### No diretório do Redox OS `~/redox-dev/redox/`

```
~/redox-dev/redox/
├── .config                       # 🔧 MODIFICAR - Adicionar flags QEMU
│
├── config/x86_64/desktop.toml    # 🔧 MODIFICAR - Adicionar eva-daemon aos pacotes
│
└── cookbook/recipes/
    └── eva-daemon/
        └── recipe.toml           # ✏️ CRIAR - Receita de build do EVA
```

---

## 📝 DETALHAMENTO DOS ARQUIVOS

### 1️⃣ `~/redox-dev/eva-daemon/Cargo.toml`

```toml
[package]
name = "eva-daemon"
version = "1.0.0"
edition = "2021"
authors = ["Jose R F Junior <teu@email.com>"]
description = "EVA AI Voice Assistant for Redox OS"
license = "MIT"

[dependencies]
tokio = { version = "1.40", features = ["rt-multi-thread", "net", "io-util", "time"], default-features = false }
tokio-tungstenite = { version = "0.20", features = ["rustls-tls-webpki-roots"] }
rustls = "0.23"
rustls-native-certs = "0.7"
rustls-pemfile = "2.0"
webpki-roots = "0.26"
tokio-rustls = "0.26"
futures-util = "0.3"
url = "2.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.22"

[target.'cfg(target_os = "redox")'.dependencies]
redox_syscall = "0.5"

[profile.release]
opt-level = "z"     # Otimizar para tamanho
lto = true          # Link-Time Optimization
codegen-units = 1   # Melhor otimização
strip = true        # Remover símbolos de debug
```

---

### 2️⃣ `~/redox-dev/eva-daemon/.gitignore`

```gitignore
# Rust
/target/
Cargo.lock
**/*.rs.bk
*.pdb

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log

# Secrets
.env
api_key.txt
```

---

### 3️⃣ `~/redox-dev/eva-daemon/README.md`

```markdown
# 🧠 EVA Daemon - AI Voice Assistant for Redox OS

Native voice AI integration using Google Gemini 2.5 Flash.

## Features
- 🎤 Real-time voice capture via `audio:` scheme
- 🔊 Audio playback with ring buffer
- 🌐 WebSocket streaming to Gemini API
- 🔐 Secure TLS 1.3 connection with rustls
- 🎯 Voice Activity Detection (VAD)

## Build

```bash
# For Linux (testing)
cargo build --release

# For Redox OS
cargo build --target x86_64-unknown-redox --release
```

## Configuration

```bash
export GEMINI_API_KEY="your_api_key_here"
```

## Usage

```bash
eva-daemon
```

## License
MIT
```

---

### 4️⃣ `~/redox-dev/eva-daemon/src/main.rs`

**(Conteúdo completo já fornecido na FASE 5 acima - 150 linhas)**

---

### 5️⃣ `~/redox-dev/eva-daemon/src/tls.rs`

**(Conteúdo completo já fornecido na FASE 2 acima - 80 linhas)**

---

### 6️⃣ `~/redox-dev/eva-daemon/src/websocket.rs`

**(Conteúdo completo já fornecido na FASE 3 acima - 90 linhas)**

---

### 7️⃣ `~/redox-dev/eva-daemon/src/audio.rs`

**(Conteúdo completo já fornecido na FASE 4 acima - 120 linhas)**

---

### 8️⃣ `~/redox-dev/eva-daemon/src/gemini.rs`

**(Conteúdo completo já fornecido na FASE 5 acima - 140 linhas)**

---

### 9️⃣ `~/redox-dev/redox/.config`

**🔧 MODIFICAR - Adicionar estas linhas no final:**

```makefile
# EVA Audio & Network Configuration
QEMU_FLAGS += -device intel-hda -device hda-duplex
QEMU_FLAGS += -audiodev pa,id=snd0
QEMU_FLAGS += -netdev user,id=net0 -device e1000,netdev=net0
```

---

### 🔟 `~/redox-dev/redox/config/x86_64/desktop.toml`

**🔧 MODIFICAR - Adicionar no final da seção `[packages]`:**

```toml
[packages]
# ... (outros pacotes já existentes)

# EVA Daemon
eva-daemon = "recipe"
```

---

### 1️⃣1️⃣ `~/redox-dev/redox/cookbook/recipes/eva-daemon/recipe.toml`

**✏️ CRIAR - Receita de build:**

```toml
[source]
git = "https://github.com/teu-username/eva-daemon.git"
branch = "main"

[build]
template = "cargo"

# Dependências do sistema Redox
dependencies = [
    "audio",
    "network"
]
```

---

## 📊 RESUMO VISUAL

```
ARQUIVOS NOVOS (11):
✏️ ~/redox-dev/eva-daemon/Cargo.toml
✏️ ~/redox-dev/eva-daemon/.gitignore
✏️ ~/redox-dev/eva-daemon/README.md
✏️ ~/redox-dev/eva-daemon/src/main.rs
✏️ ~/redox-dev/eva-daemon/src/tls.rs
✏️ ~/redox-dev/eva-daemon/src/websocket.rs
✏️ ~/redox-dev/eva-daemon/src/audio.rs
✏️ ~/redox-dev/eva-daemon/src/gemini.rs
✏️ ~/redox-dev/redox/cookbook/recipes/eva-daemon/recipe.toml

ARQUIVOS MODIFICADOS (2):
🔧 ~/redox-dev/redox/.config
🔧 ~/redox-dev/redox/config/x86_64/desktop.toml
```

---

## 🚀 SCRIPT DE CRIAÇÃO AUTOMÁTICA

Queres que eu crie um **script bash** que gera todos estes arquivos automaticamente? Seria algo assim:

```bash
#!/bin/bash
# setup-eva.sh - Cria toda a estrutura do projeto automaticamente

# Criar diretórios
mkdir -p ~/redox-dev/eva-daemon/src
mkdir -p ~/redox-dev/redox/cookbook/recipes/eva-daemon

# Criar Cargo.toml
cat > ~/redox-dev/eva-daemon/Cargo.toml << 'EOF'
[package]
name = "eva-daemon"
...
EOF

# Criar src/main.rs
cat > ~/redox-dev/eva-daemon/src/main.rs << 'EOF'
mod tls;
mod websocket;
...
EOF

# ... (etc para todos os arquivos)
```
