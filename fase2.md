# 🔐 FASE 2: Adicionar TLS/SSL com `rustls`

## 📋 Objetivo da Fase

Implementar suporte completo a TLS/SSL no EVA Daemon usando a biblioteca `rustls`, permitindo conexões seguras HTTPS e preparando o terreno para WebSocket seguro (WSS) na Fase 3.

---

## ✅ Pré-requisitos

Antes de começar esta fase, certifica-te que:

- ✅ Completaste a **Fase 1** (teste de conectividade básica)
- ✅ O EVA Daemon compila sem erros para `x86_64-unknown-redox`
- ✅ Tens o Rust nightly instalado com `rust-src`
- ✅ O teste de DNS e TCP básico funciona

---

## 🎯 Passos da Implementação

### Passo 2.1: Atualizar Dependências do Projeto

Vamos adicionar as bibliotecas necessárias para TLS/SSL.

```bash
cd ~/redox-dev/eva-daemon/
nano Cargo.toml
```

**Atualiza o `Cargo.toml` para a versão 0.2.0:**

```toml
[package]
name = "eva-daemon"
version = "0.2.0"
edition = "2021"

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
```

**O que adicionámos:**
- `rustls` - Implementação TLS em Rust puro (sem OpenSSL)
- `rustls-native-certs` - Carrega certificados CA do sistema
- `rustls-pemfile` - Parser de certificados PEM
- `webpki-roots` - Certificados raiz embutidos (fallback)
- `tokio-rustls` - Integração do rustls com Tokio

---

### Passo 2.2: Criar o Módulo TLS

Cria um novo ficheiro para gerenciar conexões TLS:

```bash
nano src/tls.rs
```

**Conteúdo completo do `src/tls.rs`:**

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

**Explicação do código:**

1. **`TlsManager::new()`**
   - Cria um armazenamento de certificados raiz
   - Carrega certificados do sistema operativo
   - Adiciona certificados embutidos como fallback
   - Configura o cliente TLS sem autenticação de cliente

2. **`TlsManager::connect()`**
   - Estabelece conexão TCP primeiro
   - Realiza o handshake TLS sobre a conexão TCP
   - Retorna um stream TLS pronto para uso

3. **Testes**
   - Valida que a conexão TLS funciona com um servidor real (Google)

---

### Passo 2.3: Atualizar o Main para Testar TLS

Agora vamos modificar o `main.rs` para usar o novo módulo TLS:

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

**O que este código faz:**

1. **Inicialização** - Cria o gerenciador TLS com certificados
2. **Conexão Segura** - Estabelece conexão TLS com google.com:443
3. **Requisição HTTP** - Envia um GET request simples
4. **Validação** - Lê e exibe as primeiras 10 linhas da resposta

---

### Passo 2.4: Testar Localmente (Linux)

Antes de compilar para Redox, testa no teu sistema Linux:

```bash
# Compilar e testar
cargo build --release
cargo test

# Se tudo funcionar, executar
./target/release/eva-daemon
```

**Saída esperada:**

```
🧠 EVA Daemon v0.2.0 - Teste TLS/SSL
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1/3] Inicializando TLS Manager...
✅ TLS Manager criado com sucesso

[2/3] Conectando a google.com:443 via TLS...
✅ Handshake TLS completo!

[3/3] Enviando requisição HTTP GET...
📥 Resposta recebida (1024 bytes):
HTTP/1.1 301 Moved Permanently
Location: https://www.google.com/
Content-Type: text/html; charset=UTF-8
...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ FASE 2 COMPLETA - TLS funcional!
```

---

### Passo 2.5: Compilar para Redox OS

Se os testes locais passarem, compila para o target do Redox:

```bash
cargo build --target x86_64-unknown-redox --release
```

**Verificar o binário:**

```bash
ls -lh target/x86_64-unknown-redox/release/eva-daemon
```

---

### Passo 2.6: Atualizar o Repositório Git

```bash
# Adicionar novos ficheiros
git add src/tls.rs
git add Cargo.toml
git add src/main.rs

# Commit
git commit -m "Fase 2: Adicionar suporte TLS/SSL com rustls"

# Push para o repositório remoto
git push origin main
```

---

## 🐛 Troubleshooting

### Erro: `rustls-native-certs` não encontra certificados

**Solução:**
```bash
# No Linux, instala certificados CA
sudo apt install ca-certificates

# Ou usa apenas webpki-roots (já incluído)
```

### Erro: `failed to verify certificate`

**Causa:** Certificados raiz não carregados corretamente

**Solução:**
```rust
// No tls.rs, força usar apenas webpki-roots:
let mut root_store = RootCertStore::empty();
root_store.extend(
    webpki_roots::TLS_SERVER_ROOTS.iter().cloned()
);
```

### Erro de compilação no Redox

**Solução:**
```bash
# Limpa o cache e recompila
cargo clean
cargo build --target x86_64-unknown-redox --release
```

---

## 📊 Checklist da Fase 2

- [ ] Atualizar `Cargo.toml` com dependências TLS
- [ ] Criar `src/tls.rs` com `TlsManager`
- [ ] Atualizar `src/main.rs` para testar TLS
- [ ] Compilar e testar no Linux
- [ ] Executar `cargo test` com sucesso
- [ ] Compilar para `x86_64-unknown-redox`
- [ ] Fazer commit e push das alterações
- [ ] Verificar que a resposta HTTP é recebida corretamente

---

## 🎓 Conceitos Aprendidos

### Por que rustls em vez de OpenSSL?

1. **Segurança** - Escrito em Rust, sem vulnerabilidades de memória
2. **Portabilidade** - Não depende de bibliotecas C do sistema
3. **Tamanho** - Binário menor e mais rápido
4. **Compatibilidade** - Funciona melhor com Redox OS

### Como funciona o handshake TLS?

```
Cliente                          Servidor
   |                                |
   |-------- ClientHello --------->|
   |                                |
   |<------- ServerHello ----------|
   |<------- Certificate ----------|
   |<----- ServerHelloDone --------|
   |                                |
   |---- ClientKeyExchange ------->|
   |---- ChangeCipherSpec -------->|
   |-------- Finished ------------>|
   |                                |
   |<--- ChangeCipherSpec ---------|
   |<------- Finished -------------|
   |                                |
   |===== Encrypted Data =========>|
```

### Certificados Raiz (Root Certificates)

- **Sistema** - Carregados de `/etc/ssl/certs` no Linux
- **Embutidos** - `webpki-roots` contém ~140 CAs confiáveis
- **Validação** - Verifica cadeia de certificados até uma CA raiz

---

## 🚀 Próximos Passos

Com TLS funcionando, estás pronto para a **Fase 3: Implementar WebSocket Client**.

Na próxima fase vais:
- Usar `tokio-tungstenite` para WebSocket
- Estabelecer conexão WSS (WebSocket Secure)
- Testar com servidor echo público
- Preparar para conectar ao Gemini API

---

## 📚 Recursos Adicionais

- [Documentação rustls](https://docs.rs/rustls/)
- [RFC 8446 - TLS 1.3](https://datatracker.ietf.org/doc/html/rfc8446)
- [webpki-roots](https://github.com/rustls/webpki-roots)
- [Redox OS Networking](https://doc.redox-os.org/book/ch04-07-networking.html)

---

**Status:** ✅ Fase 2 Completa  
**Próxima:** 🌐 Fase 3 - WebSocket Client  
**Versão EVA:** 0.2.0
