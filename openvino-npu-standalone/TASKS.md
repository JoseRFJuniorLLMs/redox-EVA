# Tarefas: Isolar Plugin NPU OpenVINO

## 📋 FASE 1: EXTRAÇÃO (2-3h)

### ✅ Task 1.1: Localizar Plugin
- [x] Encontrar `openvino_intel_npu_plugin.dll`
- [x] Local: `C:/Users/web2a/.../openvino/libs/`
- [x] Tamanho: 4.4MB

### ⏳ Task 1.2: Copiar Arquivos
```bash
# Copiar plugin NPU
cp openvino/libs/openvino_intel_npu_plugin.dll lib/

# Copiar core mínimo
cp openvino/libs/openvino.dll lib/
cp openvino/libs/openvino_c.dll lib/

# TBB (threading)
cp openvino/libs/tbb12.dll lib/
```

### ⏳ Task 1.3: Extrair Headers
```bash
# Headers NPU
cp -r openvino/include/openvino/runtime/intel_npu/ include/

# Headers core mínimos
cp openvino/include/openvino/runtime/core.hpp include/
cp openvino/include/openvino/runtime/compiled_model.hpp include/
cp openvino/include/openvino/runtime/infer_request.hpp include/
```

### ⏳ Task 1.4: Analisar Dependências
```bash
# Windows: usar dumpbin
dumpbin /dependents openvino_intel_npu_plugin.dll

# Lista esperada:
# - openvino.dll
# - tbb12.dll
# - kernel32.dll (Windows)
# - msvcrt.dll (Windows)
```

---

## 📋 FASE 2: CRIAR WRAPPER (4-6h)

### ⏳ Task 2.1: API Standalone
**Arquivo:** `src/npu_plugin.cpp`

```cpp
#include "npu_plugin.h"
#include <openvino/openvino.hpp>

class NPUPluginStandalone {
private:
    ov::Core core;
    ov::CompiledModel compiled_model;

public:
    void initialize() {
        // Força uso do plugin NPU apenas
        core.set_property("NPU", {{"DEVICE_ID", "0"}});
    }

    void* compile(const char* model_path) {
        auto model = core.read_model(model_path);
        compiled_model = core.compile_model(model, "NPU");
        return &compiled_model;
    }

    void* execute(void* compiled, void* input) {
        auto infer_request = compiled_model.create_infer_request();
        // TODO: set input tensors
        infer_request.infer();
        // TODO: get output tensors
        return nullptr;
    }
};
```

### ⏳ Task 2.2: API C (FFI)
**Arquivo:** `src/npu_plugin_c.cpp`

```cpp
extern "C" {

typedef void* npu_plugin_t;
typedef void* npu_compiled_model_t;

npu_plugin_t npu_plugin_create() {
    return new NPUPluginStandalone();
}

void npu_plugin_destroy(npu_plugin_t plugin) {
    delete static_cast<NPUPluginStandalone*>(plugin);
}

npu_compiled_model_t npu_compile(
    npu_plugin_t plugin,
    const char* model_path
) {
    auto p = static_cast<NPUPluginStandalone*>(plugin);
    return p->compile(model_path);
}

int npu_execute(
    npu_plugin_t plugin,
    npu_compiled_model_t compiled,
    float* input,
    int input_size,
    float* output,
    int output_size
) {
    // TODO: implementar
    return 0;
}

} // extern "C"
```

### ⏳ Task 2.3: Integração ONNX Runtime
**Arquivo:** `src/onnxrt_npu_provider.cpp`

Substituir o provider customizado que criamos por um que usa o plugin isolado:

```cpp
#include "npu_plugin.h"
#include "rodox_npu_execution_provider.h"

Status RodoxNPUExecutionProvider::InitializeNPU() {
    // Usar plugin standalone ao invés de OpenVINO full
    npu_device_ = npu_plugin_create();
    return Status::OK();
}
```

### ⏳ Task 2.4: CMake Build
**Arquivo:** `cmake/CMakeLists.txt`

```cmake
cmake_minimum_required(VERSION 3.15)
project(OpenVINO_NPU_Standalone)

# Apenas plugin NPU
add_library(openvino_npu_standalone SHARED
    src/npu_plugin.cpp
    src/npu_plugin_c.cpp
    src/onnxrt_npu_provider.cpp
)

# Link apenas contra plugin NPU
target_link_libraries(openvino_npu_standalone
    ${CMAKE_CURRENT_SOURCE_DIR}/lib/openvino_intel_npu_plugin.dll
    ${CMAKE_CURRENT_SOURCE_DIR}/lib/openvino.dll
    ${CMAKE_CURRENT_SOURCE_DIR}/lib/tbb12.dll
)

# Headers
target_include_directories(openvino_npu_standalone PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)
```

---

## 📋 FASE 3: TESTES (2-3h)

### ⏳ Task 3.1: Teste Básico
**Arquivo:** `tests/test_basic.cpp`

```cpp
#include "npu_plugin.h"
#include <cassert>

int main() {
    auto plugin = npu_plugin_create();
    assert(plugin != nullptr);

    // TODO: compilar modelo simples
    // TODO: executar inferência

    npu_plugin_destroy(plugin);
    return 0;
}
```

### ⏳ Task 3.2: Teste com ONNX Runtime
```cpp
// Testar provider com modelo YOLO
Ort::SessionOptions options;
options.AppendExecutionProvider("RodoxNPU", {});

Ort::Session session(env, "yolo.onnx", options);
// Verificar que usa NPU
```

### ⏳ Task 3.3: Benchmark
```bash
# Comparar:
# - OpenVINO full vs Plugin isolado
# - Latência
# - Throughput
# - Memória usada
```

---

## 📋 FASE 4: OTIMIZAÇÃO (2-3h)

### ⏳ Task 4.1: Reduzir Tamanho
```bash
# Strip símbolos desnecessários
strip --strip-unneeded lib/*.dll

# Comprimir com UPX
upx --best lib/*.dll
```

### ⏳ Task 4.2: Remover Dependências Não Usadas
- Analisar quais símbolos do `openvino.dll` são realmente usados
- Criar `openvino_minimal.dll` com apenas o necessário

### ⏳ Task 4.3: Portar TBB para Rust
```rust
// Substituir TBB (C++) por rayon (Rust)
use rayon::prelude::*;

pub fn parallel_for<F>(start: usize, end: usize, f: F)
where
    F: Fn(usize) + Sync + Send,
{
    (start..end).into_par_iter().for_each(f);
}
```

---

## 📋 FASE 5: REDOX PORT (3-4h)

### ⏳ Task 5.1: Substituir Windows APIs
- `CreateThread` → Rust `std::thread`
- `WaitForSingleObject` → Rust `Condvar`
- `LoadLibrary` → Redox dynamic linking

### ⏳ Task 5.2: Recompilar para Redox
```bash
cargo build --target x86_64-unknown-redox --release
```

### ⏳ Task 5.3: Integrar com Driver EVA-OS
```rust
// driver-c-api/src/lib.rs
#[link(name = "openvino_npu_standalone")]
extern "C" {
    fn npu_plugin_create() -> *mut c_void;
    fn npu_compile(...) -> ...;
}
```

---

## 🎯 Meta Final

**Tamanho:**
- ❌ OpenVINO Full: 1.75GB
- ✅ Plugin Isolado: ~50MB

**Funcionalidade:**
- ✅ Compilar ONNX → NPU IR
- ✅ Executar na NPU
- ✅ Integração ONNX Runtime
- ✅ Portável para Redox

**Tempo Estimado:** 13-18 horas total

---

## Próximos Passos

1. **Agora:** Copiar arquivos (Task 1.2)
2. **Depois:** Criar wrapper C (Task 2.2)
3. **Final:** Testar com Qwen modelo
