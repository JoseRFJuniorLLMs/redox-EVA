// Test básico do plugin NPU standalone

#include "../include/npu_plugin.h"
#include <iostream>
#include <vector>

int main() {
    std::cout << "🔍 Testando OpenVINO NPU Plugin Standalone\n\n";

    // Create plugin
    std::cout << "1. Criando plugin NPU...\n";
    auto plugin = npu_plugin_create();

    if (!plugin) {
        std::cerr << "❌ Falha ao criar plugin: " << npu_plugin_get_error() << "\n";
        return 1;
    }
    std::cout << "✅ Plugin criado\n\n";

    // TODO: Compile model (precisa de modelo ONNX)
    // std::cout << "2. Compilando modelo...\n";
    // auto compiled = npu_plugin_compile(plugin, "model.onnx");

    // TODO: Execute inference
    // std::vector<float> input(1000, 1.0f);
    // std::vector<float> output(1000);
    // npu_plugin_execute(plugin, compiled, input.data(), ...);

    // Cleanup
    npu_plugin_destroy(plugin);

    std::cout << "✅ Teste básico concluído!\n";
    return 0;
}
