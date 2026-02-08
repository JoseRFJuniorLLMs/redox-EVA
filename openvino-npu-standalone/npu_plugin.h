// Copyright (c) OpenVINO NPU Standalone
// Licensed under MIT

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>

// Opaque handles
typedef void* npu_plugin_t;
typedef void* npu_compiled_model_t;

/**
 * @brief Create NPU plugin instance
 */
npu_plugin_t npu_plugin_create();

/**
 * @brief Destroy plugin instance
 */
void npu_plugin_destroy(npu_plugin_t plugin);

/**
 * @brief Compile ONNX model for NPU
 */
npu_compiled_model_t npu_plugin_compile(
    npu_plugin_t plugin,
    const char* model_path
);

/**
 * @brief Execute inference on NPU
 */
int npu_plugin_execute(
    npu_plugin_t plugin,
    npu_compiled_model_t compiled,
    const float* input,
    size_t input_size,
    float* output,
    size_t output_size
);

/**
 * @brief Free compiled model
 */
void npu_plugin_free_model(npu_compiled_model_t compiled);

/**
 * @brief Get last error message
 */
const char* npu_plugin_get_error();

#ifdef __cplusplus
}
#endif
