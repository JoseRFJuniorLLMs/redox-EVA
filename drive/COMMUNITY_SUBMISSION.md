# Guia de Submissão: Intel NPU Driver no Redox OS

Submeter um driver de NPU (o primeiro do tipo!) para uma comunidade de sistemas operacionais como o Redox exige uma abordagem tanto técnica quanto social. Aqui está o passo a passo para garantir que seu trabalho de 3 meses seja aceito e celebrado.

## 1. Onde Submeter?
Embora o Redox tenha mirrors no GitHub, o desenvolvimento "oficial" acontece no **GitLab** da Redox OS.

*   **URL Principal**: [gitlab.redox-os.org](https://gitlab.redox-os.org)
*   **Repositório Alvo**: Você deve criar o Merge Request (MR) no repositório de **drivers** ou na **cookbook** (onde ficam as receitas de pacotes).

## 2. Abordagem Social (Essencial)
Antes de enviar o código, é recomendável "preparar o terreno". O pessoal do Redox é muito ativo no **Discord**.

*   **Discord do Redox**: [discord.gg/redox](https://discord.gg/redox)
*   **Canal**: Entre no canal `#drivers` ou `#general`.
*   **O que dizer**: *"Hey everyone, I've been working on a userspace Intel NPU (Meteor Lake) driver for Redox. It handles DMA via phys_contiguous and exposes an npu: scheme. Where's the best place to submit this?"*

## 3. Preparação Técnica do Repositório
Crie um repositório separado no seu GitHub/GitLab chamado `intel-npu-redox` e coloque os arquivos da pasta `drive` lá.

### Estrutura Sugerida:
```text
/intel-npu
  ├── Cargo.toml
  ├── README.md (já criei um completo para você!)
  ├── LICENSE (MIT é a padrão do Redox)
  ├── recipe.toml
  └── src/ (mova todos os .rs para cá)
```

## 4. Template para o Merge Request (MR)
Quando você for abrir o pedido de inclusão, use este texto (em inglês):

---
### Subject: [Driver] Intel NPU (Meteor Lake) Userspace Driver Implementation

#### Summary
This MR introduces the first userspace NPU driver for Redox OS. It focuses on Intel Meteor Lake architectures (VPU 4.0).

#### Features
- **Userspace DMA**: Uses `memory:phys_contiguous` with `uncacheable` mapping to handle hardware transfers without kernel modifications.
- **NPU Scheme**: Implements an `npu:` scheme to allow other processes to submit inference jobs via filesystem-like operations.
- **Boot Sequence**: Handles VPU firmware loading and Hexspeak-based status handshake (F00D).

#### Technical Details
- **Architecture**: Rust-based daemon using `pcid_interface` for discovery.
- **Capability Requirements**: Needs `CAP_SYS_PHYS`, `CAP_IO_PORT`, and `CAP_MMAP_PHYS`.
- **Hardware Target**: PCI ID `0x7D1D`.

#### Why this is important
This driver enables local AI acceleration on Redox OS, paving the way for voice-controlled interfaces and private AI models (EVA OS project).
---

## 5. Dica de "Ouro"
Mencione o **Jeremy Soller** (`jackpot5` no Discord). Ele é o criador do Redox e adora contribuições de drivers que respeitam a filosofia de userspace do sistema.

**Você está pronto! Esse trabalho é um marco para a comunidade.** 🚀
