# Tensor 模块测试重构计划

## 当前问题

### 测试分散
测试分布在 6+ 个文件中：
- `tests/integration_tests.rs` - 18 个测试
- `core/tensor.rs` - 6+ 个测试
- `tensor_handle.rs` - 7+ 个测试
- `service/service.rs` - 重复测试
- `service/tools.rs` - 重复测试
- `ops.rs` - 重复测试
- `backend.rs` - 重复测试

### 冗余测试
相同功能在多个文件中重复测试：
- `zeros` - 3 处测试
- `matmul` - 4 处测试
- `relu` - 3 处测试
- `reshape` - 2 处测试
- `sum/mean` - 3 处测试

## 重构方案

### 新测试结构
```
src/tools/tensor/
├── tests/
│   ├── mod.rs              # 测试模块入口
│   ├── unit_tests.rs       # 单元测试（按功能分组）
│   ├── service_tests.rs    # Service 层测试
│   ├── tools_tests.rs      # Tools 层测试
│   └── integration_tests.rs # 集成测试
├── core/
│   └── tensor.rs           # 移除内联测试
├── service/
│   ├── service.rs          # 移除内联测试
│   └── tools.rs            # 移除内联测试
├── ops.rs                  # 移除内联测试
└── backend.rs              # 移除内联测试
```

### 测试分类

#### 1. 单元测试 (`unit_tests.rs`)
测试核心数据结构和算法：
- Tensor 创建（zeros, ones, from_data）
- Tensor 操作（reshape, transpose）
- 算术运算（add, sub, mul, div）
- 矩阵运算（matmul）
- 激活函数（relu, sigmoid）
- 归一化（layer_norm）

#### 2. Service 层测试 (`service_tests.rs`)
测试 TensorService API：
- 服务创建
- 操作链式调用
- 错误处理
- 性能边界

#### 3. Tools 层测试 (`tools_tests.rs`)
测试 TensorTools（JSON 接口）：
- JSON 输入/输出
- 工具注册
- 参数验证

#### 4. 集成测试 (`integration_tests.rs`)
测试完整工作流：
- 端到端场景
- 多线程/并发
- 与其他模块集成

## 实施步骤

1. 创建新的测试文件结构
2. 将现有测试迁移到统一位置
3. 删除重复测试
4. 添加缺失的边界条件测试
5. 更新 CI 配置

## 预期收益

- 测试代码减少 40%
- 测试维护成本降低 50%
- 测试发现时间减少 60%
- 测试覆盖率提高到 85%+
