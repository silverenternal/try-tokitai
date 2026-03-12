# Kafka 消费者组

## 核心概念

### 消费者组（Consumer Group）

- 一组消费者实例，共同消费一个或多个 topic
- 每个分区只能被组内的一个消费者消费
- 不同组可以同时消费同一 topic（发布订阅模式）

### 分区分配策略

| 策略 | 说明 |
|------|------|
| Range | 按分区范围分配，可能导致不均衡 |
| RoundRobin | 轮询分配，更均衡 |
| Sticky | 粘性分配，rebalance 时尽量保持原分配 |
| CooperativeSticky | 协作式粘性，增量 rebalance |

## Rebalance（重平衡）

### 触发条件

1. 消费者加入或离开组
2. Topic 分区数变化
3. 消费者超时（session.timeout.ms）

### Rebalance 问题

**Stop-the-world：** 所有消费者停止消费

**解决方案：**
- 使用 CooperativeSticky 策略
- 调整心跳和超时参数
- 实现静态成员（group.instance.id）

## 偏移量管理

### 提交方式

```java
// 自动提交（不推荐，可能丢失或重复）
props.put("enable.auto.commit", "false");

// 同步提交
consumer.commitSync();

// 异步提交
consumer.commitAsync((offsets, exception) -> {
    if (exception != null) {
        // 处理失败
    }
});
```

### 精确一次语义

```java
// Kafka 2.5+ 支持
props.put("isolation.level", "read_committed");
props.put("enable.idempotence", "true");
```

## 常见问题

### 消费积压

**原因：**
- 消费速度慢于生产速度
- 消费者宕机

**解决方案：**
- 增加消费者实例
- 优化消费逻辑
- 临时扩容：增加分区和消费者

### 重复消费

**原因：**
- Rebalance 导致偏移量提交失败
- 消费者处理完成后未及时提交

**解决方案：**
- 实现幂等性
- 使用事务消息
