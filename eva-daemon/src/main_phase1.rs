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
