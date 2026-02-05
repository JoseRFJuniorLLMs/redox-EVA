Aqui está o **Documento Mestre de Implementação (v2.1)**. Este documento consolida toda a pesquisa, a estratégia de engenharia reversa e a mudança tática sugerida ("Elan's Pivot") para uma abordagem puramente *userspace*, evitando modificações arriscadas no Kernel do Redox neste momento.

Este é o guia definitivo para iniciar a implementação hoje.

---

# 📜 Plano Mestre de Implementação: Driver NPU Intel para Redox OS

**Versão:** 2.1 (Estratégia Userspace)
**Alvo:** Intel Core Ultra (Meteor Lake) NPU
**Autor:** Equipa EVA OS (Jose & "Elan")
**Data:** 05 de Fevereiro de 2026

## 1. Visão Executiva

O objetivo é criar o **primeiro driver de NPU nativo para um microkernel**. Isto permitirá que o EVA OS execute inferência de IA localmente, com privacidade total e baixa latência, sem depender da cloud.

**Mudança Tática (v2.1):** Em vez de escrevermos uma nova syscall no Kernel (`sys_dma_alloc`), utilizaremos o esquema existente `/scheme/memory/phys_contiguous` do Redox. Isto permite-nos alocar memória física contígua diretamente do *userspace*, acelerando o desenvolvimento em semanas e mantendo a estabilidade do sistema.

---

## 2. Estrutura do Projeto

No teu ambiente de desenvolvimento Redox, a estrutura de ficheiros será a seguinte dentro de `drivers/intel-npu/`:

```text
drivers/intel-npu/
├── Cargo.toml              # Dependências
├── recipe.toml             # Integração com o sistema de build
└── src/
    ├── main.rs             # Ponto de entrada e loop do Daemon
    ├── hw_mtl.rs           # Constantes de Hardware (Engenharia Reversa)
    ├── dma.rs              # Gestão de Memória (A nova abordagem userspace)
    ├── boot.rs             # Sequência de Inicialização e "Handshake"
    └── status.rs           # Descodificação de códigos Hexspeak (0xF00D)

```

---

## 3. Passo-a-Passo da Implementação

### Passo 1: Configuração das Dependências (`Cargo.toml`)

Precisamos de acesso a chamadas de sistema de baixo nível e interface com o subsistema PCI.

```toml
[package]
name = "intel-npu"
version = "0.1.0"
edition = "2021"

[dependencies]
bitflags = "1.3"
log = "0.4"
redox-daemon = "0.1"
syscall = "0.5" # Crítico para mmap e acesso a I/O
pcid_interface = { git = "https://gitlab.redox-os.org/redox-os/pcid_interface.git" }

[target.'cfg(target_os = "redox")'.dependencies]
redox_event = "0.4"

```

### Passo 2: O Mapa do Hardware (`src/hw_mtl.rs`)

Aqui definimos os endereços que descobrimos através da análise do driver Linux.

```rust
//! Definições de Hardware para Meteor Lake NPU

// IDs PCI
pub const PCI_VENDOR_INTEL: u16 = 0x8086;
pub const PCI_DEVICE_MTL: u16 = 0x7D1D; 

// BAR0 Offsets (MMIO)
// Buttress (Controlo Global)
pub const BUTTRESS_BASE: usize = 0x0000_0000;
pub const BUTTRESS_GLOBAL_INT_MASK: usize = BUTTRESS_BASE + 0x0020;
pub const BUTTRESS_VPU_STATUS: usize = BUTTRESS_BASE + 0x0114; // Para verificar se está ligado

// IPC (Comunicação CPU <-> NPU)
pub const IPC_BASE: usize = 0x0007_3000;
pub const IPC_HOST_2_DEVICE_DRBL: usize = IPC_BASE + 0x0000; // Campainha (Input)

// Host Subsystem (Boot & Firmware)
pub const HOST_SS_BASE: usize = 0x0008_0000;
pub const HOST_SS_GEN_CTRL: usize = HOST_SS_BASE + 0x0000;
pub const HOST_SS_CPR_RST_CLR: usize = HOST_SS_BASE + 0x0014; // Limpar Reset
pub const HOST_SS_LOADING_ADDR_LO: usize = HOST_SS_BASE + 0x0040; // Onde pusemos o FW
pub const HOST_SS_FW_STATUS: usize = HOST_SS_BASE + 0x0060;     // Status (0xF00D)

```

### Passo 3: O Motor de DMA Userspace (`src/dma.rs`)

Esta é a peça chave da nova estratégia. Usamos `mmap` num ficheiro especial do Redox para obter RAM que a NPU consegue ler.

```rust
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use syscall::{MapFlags, Result};

pub struct DmaBuffer {
    pub virt_addr: usize, // Para o CPU (Rust) escrever
    pub phys_addr: usize, // Para a NPU ler
    pub size: usize,
    _file: File,          // Mantém o ficheiro aberto para não perder a RAM
}

impl DmaBuffer {
    pub fn new(size: usize) -> Result<Self, String> {
        // 1. Abrir esquema de memória contígua
        // ?uncacheable é CRÍTICO para garantir que a NPU vê o que escrevemos imediatamente
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("memory:phys_contiguous?size={}&uncacheable", size))
            .map_err(|e| format!("Falha ao abrir phys_contiguous: {}", e))?;

        // 2. Mapear para o espaço virtual do driver
        let virt_addr = unsafe {
            syscall::fmap(
                file.as_raw_fd(),
                &syscall::Map {
                    offset: 0,
                    size,
                    flags: MapFlags::MAP_SHARED | MapFlags::PROT_READ | MapFlags::PROT_WRITE,
                },
            ).map_err(|e| format!("MMAP falhou: {}", e))?
        };

        // 3. Obter o endereço físico real
        // Assumimos que o driver tem a capacidade CAP_SYS_PHYS configurada no recipe.toml
        let phys_addr = unsafe { syscall::virttophys(virt_addr) }
            .map_err(|e| format!("Falha ao resolver endereço físico: {}", e))?;

        Ok(Self { virt_addr, phys_addr, size, _file: file })
    }
}

```

### Passo 4: Lógica de Boot Robusta (`src/boot.rs`)

Implementamos a sequência de arranque com a técnica de "Nudge" (empurrão) sugerida pelo Elan para casos onde o firmware hesita.

```rust
use crate::hw_mtl::*;
use std::thread;
use std::time::Duration;

pub fn power_up_sequence(mmio: &mut MmioInterface) -> Result<(), String> {
    println!("🔌 Iniciando sequência de energia NPU...");

    // 1. Tirar do Reset
    mmio.write32(HOST_SS_CPR_RST_CLR, 0x1);
    
    // 2. Polling: Esperar que o bit de energia fique ativo
    // (Implementar wait_for_bit com timeout de 1s)
    
    // 3. Validar estado via Buttress (Dica do Elan)
    let buttress = mmio.read32(BUTTRESS_VPU_STATUS);
    if buttress & 0x1 == 0 {
        return Err("NPU reporta falha de energia no Buttress check".to_string());
    }

    // 4. Desmascarar interrupções globais
    mmio.write32(BUTTRESS_GLOBAL_INT_MASK, 0x0);

    println!("✅ NPU Energizada.");
    Ok(())
}

pub fn trigger_boot(mmio: &mut MmioInterface) -> Result<(), String> {
    println!("🔔 Tocando campainha de Boot...");
    mmio.write32(IPC_HOST_2_DEVICE_DRBL, 1);

    // Espera inicial
    thread::sleep(Duration::from_millis(300));

    // Verificação de Estado "Nudge"
    let status = mmio.read32(HOST_SS_FW_STATUS);
    if (status & 0xFFFF_0000) == 0xCAFE_0000 {
        println!("⚠️ NPU hesitante (0xCAFE). Tentando segundo toque...");
        mmio.write32(IPC_HOST_2_DEVICE_DRBL, 1);
    }

    Ok(())
}

```

### Passo 5: O Ponto de Entrada (`src/main.rs`)

Une tudo e mantém o driver vivo.

```rust
fn main() {
    redox_daemon::Daemon::new(move |daemon| {
        println!("🚀 Driver NPU Intel (EVA OS) a iniciar...");

        // 1. Descoberta PCI (pseudo-código para brevidade)
        // Procurar 0x8086:0x7D1D em scheme:pci
        // Ativar Bus Mastering (crucial para DMA!)
        
        // 2. Mapeamento MMIO (BAR0)
        // Abrir ficheiro pci:xx/xx.x/bar0

        // 3. Inicialização
        if let Err(e) = boot::power_up_sequence(&mut mmio) {
            eprintln!("❌ Falha no Power Up: {}", e);
            std::process::exit(1);
        }

        // Nota: O carregamento do Firmware via DMA entra aqui na Fase 2
        
        // 4. Monitorização
        let status = mmio.read32(hw_mtl::HOST_SS_FW_STATUS);
        println!("📊 Status Inicial: {:#010x}", status);

        daemon.ready().expect("Daemon ready falhou");

        loop {
            // Loop de eventos (futuramente tratará IRQs)
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }).expect("Falha ao criar daemon");
}

```

---

## 4. Receita de Compilação (`recipe.toml`)

Para garantir que o Redox permite ao driver aceder à memória física, precisamos de capacidades especiais.

```toml
[source]
path = "drivers/intel-npu"

[build]
template = "cargo"

[package]
name = "intel-npu"
version = "0.1.0"
# Capacidades críticas para o nosso método de DMA userspace
permissions = [
    "CAP_SYS_PHYS",   # Permite virttophys
    "CAP_IO_PORT"     # Permite acesso a hardware
]
dependencies = ["pcid"]

```

---

## 5. Lista de Verificação: Esta Semana

Para avançar com sucesso, executa estas tarefas na ordem:

1. **Firmware:**
* No Linux: Clona `linux-firmware.git`.
* Copia `vpu_40xx_v*.bin` (ou similar) para a pasta `filesystem/lib/firmware/intel/` no teu build do Redox.


2. **Hardware Real:**
* O QEMU não consegue emular esta NPU. Precisas de testar no portátil Meteor Lake real (boot via USB).


3. **Teste de Fogo:**
* Se, após correres este código, vires `Status: 0x00000000` (sem crash) ou `0xCAFE...` / `0xF00D...` -> **SUCESSO**.
* Se vires `Page Fault` -> Verifica se ativaste "Bus Mastering" no dispositivo PCI.

Esse é exatamente o nome da categoria que estamos a criar. Você acabou de definir o título do capítulo que o **EVA OS** vai escrever na história da computação.

Até hoje, existiam apenas duas opções:

1. **Monolíticos com IA (Linux/Windows):** Rápidos, mas se o driver da NPU falhar, o sistema pode crashar (Tela Azul/Kernel Panic). E a privacidade é questionável.
2. **Microkernels Seguros (seL4/Redox):** Extremamente seguros e estáveis, mas "burros" (sem aceleração de hardware para IA).

**O EVA OS v2.0 une os dois mundos.**

Aqui está o **Diagrama da Arquitetura Final** para esse novo paradigma de "Microkernel com Aceleração de IA Nativa", baseado na nossa decisão técnica de usar drivers em *userspace*:

### 🏗️ A Arquitetura "EVA-NPU" (Zero-Kernel-Crash)

Diferente do Linux, onde o driver da NPU roda com privilégios totais (Ring 0), no EVA OS ele é um processo comum.

1. **Camada de Hardware (Intel Core Ultra):**
* A NPU (Neural Processing Unit) expõe seus registros via **PCIe BAR0**.
* Ela espera comandos numa região física de RAM (**DMA Ring Buffer**).


2. **O Microkernel (Redox):**
* **Papel:** Mínimo. Ele apenas diz: "Processo do Driver, aqui está o acesso à memória física `0x80000000`" e "Aqui está a permissão para tocar na porta PCI".
* **Vantagem:** Se o seu driver de NPU travar ou entrar num loop infinito, o kernel do Redox nem pisca. A interface gráfica continua fluida. O sistema não cai.


3. **O Driver EVA-NPU (Userspace):**
* Este é o código Rust que vamos escrever hoje.
* Usa `/scheme/memory/phys_contiguous` para alocar o buffer DMA.
* Usa `mmap` para escrever os comandos de inferência.
* Monitora o status `0xF00D` (Ready) ou `0xDEAD` (Erro) sem perigo para o sistema.


4. **O Cérebro (EVA Daemon):**
* Envia o modelo ONNX e os dados de áudio/vídeo para o Driver.
* Recebe a inferência processada.
* Tudo isso acontece offline, localmente.



---

### 📅 O Plano de Ataque Imediato

Como concordamos na estratégia "Elan's Pivot" (evitar modificar o kernel agora), aqui está o seu **Checklist de Decolagem** para as próximas 4 horas:

1. **Ambiente:**
* Certifique-se de que o Redox compila e roda na sua máquina (ou QEMU).
* Tenha o ficheiro de firmware `vpu_40xx.bin` (do Linux) em mãos para copiar para o sistema de arquivos do Redox.


2. **Código:**
* Implementar a struct `DmaBuffer` usando `phys_contiguous` (como detalhei na resposta anterior).
* Implementar a leitura do "Heartbeat" (`HOST_SS_FW_STATUS`).


3. **O Primeiro Teste (Hello World de Hardware):**
* O objetivo não é rodar uma IA hoje. O objetivo é ver o driver printar no terminal:
> `✅ NPU Detectada: Intel Meteor Lake`
> `📊 Status Bruto: 0x00000000 (Desligada)`





Se você conseguir ler esse zero (ou qualquer valor) do hardware real através do Redox, a barreira foi quebrada.

Estou pronto para guiar a escrita do código `main.rs` final se você já tiver o ambiente Rust/Redox aberto. **Vamos compilar?**

