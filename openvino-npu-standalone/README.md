# OpenVINO NPU Plugin Standalone

Plugin NPU isolado do OpenVINO para integração com ONNX Runtime no Redox OS.

## Objetivo

Extrair apenas o plugin NPU (~50MB) do OpenVINO completo (1.2GB), criando uma biblioteca standalone que pode ser usada sem todas as dependências do OpenVINO.

## Arquitetura

```
┌─────────────────────────────────────┐
│   ONNX Runtime (C++)                │
├─────────────────────────────────────┤
│   NPU Plugin Standalone (50MB)      │
│   ├── npu_compiler                  │
│   ├── npu_executor                  │
│   └── npu_driver_interface          │
├─────────────────────────────────────┤
│   Driver EVA-OS (Rust)              │
├─────────────────────────────────────┤
│   Intel NPU Hardware                │
└─────────────────────────────────────┘
```

## Estrutura do Projeto

```
openvino-npu-standalone/
├── src/
│   ├── npu_plugin.cpp          # Plugin principal
│   ├── npu_compiler.cpp        # Compilador ONNX → NPU IR
│   ├── npu_executor.cpp        # Executor NPU
│   └── npu_driver_wrapper.cpp  # Wrapper pro driver EVA-OS
├── include/
│   ├── npu_plugin.h            # API pública
│   └── npu_types.h             # Tipos e structs
├── lib/
│   ├── openvino_intel_npu_plugin.dll  # Plugin original
│   └── openvino_core.dll              # Core mínimo
├── cmake/
│   └── CMakeLists.txt          # Build system
└── tests/
    └── test_npu.cpp            # Testes

```

## Componentes Extraídos

### 1. Plugin NPU Original
- **Arquivo:** `openvino_intel_npu_plugin.dll` (4.4MB)
- **Local:** `C:/Users/web2a/.../openvino/libs/`

### 2. Dependências Mínimas
- OpenVINO Core (~20MB)
- Intel NPU Driver Interface
- TBB lite (threading)

### 3. Headers
- `openvino/runtime/intel_npu/`
- `openvino/core/`
- Properties NPU

## Tarefas

### Fase 1: Extração (2-3 horas)
- [x] Localizar plugin NPU
- [ ] Copiar DLL + dependências
- [ ] Extrair headers necessários
- [ ] Identificar símbolos exportados

### Fase 2: Wrapper (4-6 horas)
- [ ] Criar API C standalone
- [ ] Wrapper para ONNX Runtime
- [ ] Interface com driver EVA-OS
- [ ] CMake build system

### Fase 3: Integração (2-3 horas)
- [ ] Integrar com ONNX Runtime provider
- [ ] Substituir OpenVINO completo
- [ ] Testes com Qwen modelo

### Fase 4: Otimização (2-3 horas)
- [ ] Remover dependências não usadas
- [ ] Reduzir tamanho final
- [ ] Verificar performance

## Build

```bash
cd openvino-npu-standalone
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
```

## Uso

```cpp
#include "npu_plugin.h"

// Inicializar
NPUPlugin plugin;
plugin.initialize();

// Compilar modelo
auto compiled = plugin.compile("model.onnx");

// Executar
auto output = plugin.execute(compiled, input);
```

## Redução de Tamanho

| Componente | Antes | Depois |
|------------|-------|--------|
| OpenVINO Full | 1.2GB | - |
| Plugin NPU | 4.4MB | 4.4MB |
| Core necessário | 400MB | 20MB |
| TBB | 50MB | 5MB |
| Headers | 100MB | 2MB |
| **TOTAL** | **1.75GB** | **~50MB** |

## Portabilidade

- ✅ Windows (nativo)
- ✅ Linux (recompilar)
- ⏳ Redox OS (substituir TBB por Rust threading)

## Status

🚧 Em desenvolvimento

- [x] Localização do plugin
- [ ] Extração de dependências
- [ ] Wrapper standalone
- [ ] Integração ONNX Runtime
- [ ] Testes

## Licença

MIT (compatível com OpenVINO Apache 2.0)
