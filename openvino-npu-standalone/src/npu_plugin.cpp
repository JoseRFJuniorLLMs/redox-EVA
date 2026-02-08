// Copyright (c) OpenVINO NPU Standalone
// Licensed under MIT

#include "../npu_plugin.h"
#include "../include/openvino/runtime/core.hpp"
#include "../include/openvino/runtime/compiled_model.hpp"
#include "../include/openvino/runtime/infer_request.hpp"
#include <string>
#include <memory>
#include <exception>

using namespace ov;

// Global error storage
static thread_local std::string g_last_error;

static void set_error(const std::string& msg) {
    g_last_error = msg;
}

struct NPUPluginImpl {
    ov::Core core;
    std::unique_ptr<ov::CompiledModel> compiled_model;
    std::unique_ptr<ov::InferRequest> infer_request;

    NPUPluginImpl() {
        try {
            // Force NPU device only
            auto available_devices = core.get_available_devices();

            bool npu_found = false;
            for (const auto& device : available_devices) {
                if (device.find("NPU") != std::string::npos) {
                    npu_found = true;
                    break;
                }
            }

            if (!npu_found) {
                throw std::runtime_error("NPU device not found");
            }
        } catch (const std::exception& e) {
            set_error(std::string("NPU initialization failed: ") + e.what());
            throw;
        }
    }
};

extern "C" {

npu_plugin_t npu_plugin_create() {
    try {
        return new NPUPluginImpl();
    } catch (const std::exception& e) {
        set_error(e.what());
        return nullptr;
    }
}

void npu_plugin_destroy(npu_plugin_t plugin) {
    if (plugin) {
        delete static_cast<NPUPluginImpl*>(plugin);
    }
}

npu_compiled_model_t npu_plugin_compile(
    npu_plugin_t plugin,
    const char* model_path
) {
    if (!plugin || !model_path) {
        set_error("Invalid parameters");
        return nullptr;
    }

    try {
        auto impl = static_cast<NPUPluginImpl*>(plugin);

        // Read ONNX model
        auto model = impl->core.read_model(model_path);

        // Compile for NPU
        impl->compiled_model = std::make_unique<ov::CompiledModel>(
            impl->core.compile_model(model, "NPU")
        );

        // Create infer request
        impl->infer_request = std::make_unique<ov::InferRequest>(
            impl->compiled_model->create_infer_request()
        );

        return impl->compiled_model.get();

    } catch (const std::exception& e) {
        set_error(std::string("Compilation failed: ") + e.what());
        return nullptr;
    }
}

int npu_plugin_execute(
    npu_plugin_t plugin,
    npu_compiled_model_t compiled,
    const float* input,
    size_t input_size,
    float* output,
    size_t output_size
) {
    if (!plugin || !compiled || !input || !output) {
        set_error("Invalid parameters");
        return -1;
    }

    try {
        auto impl = static_cast<NPUPluginImpl*>(plugin);

        if (!impl->infer_request) {
            set_error("No infer request created");
            return -1;
        }

        // Get input tensor
        auto input_tensor = impl->infer_request->get_input_tensor(0);

        // Copy input data
        float* input_data = input_tensor.data<float>();
        size_t copy_size = std::min(input_size, input_tensor.get_byte_size());
        std::memcpy(input_data, input, copy_size);

        // Run inference
        impl->infer_request->infer();

        // Get output tensor
        auto output_tensor = impl->infer_request->get_output_tensor(0);
        float* output_data = output_tensor.data<float>();

        // Copy output data
        copy_size = std::min(output_size, output_tensor.get_byte_size());
        std::memcpy(output, output_data, copy_size);

        return 0;

    } catch (const std::exception& e) {
        set_error(std::string("Execution failed: ") + e.what());
        return -1;
    }
}

void npu_plugin_free_model(npu_compiled_model_t compiled) {
    // Model is managed by NPUPluginImpl, no action needed
}

const char* npu_plugin_get_error() {
    return g_last_error.c_str();
}

} // extern "C"
