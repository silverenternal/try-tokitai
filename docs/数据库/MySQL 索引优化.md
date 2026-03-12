# MySQL 索引优化

## 最佳实践

### 1. 选择高选择性列建立索引

选择性越高的列越适合建立索引。例如：
- 用户 ID、订单 ID 等唯一或接近唯一的列
- 避免在性别、状态等低选择性列上建立索引

### 2. 使用覆盖索引减少回表

覆盖索引是指查询所需的所有列都在索引中，无需回表查询：

```sql
-- 好的示例：覆盖索引
CREATE INDEX idx_user_email ON users(email, name);
SELECT email, name FROM users WHERE email = 'test@example.com';

-- 不好的示例：需要回表
SELECT * FROM users WHERE email = 'test@example.com';
```

### 3. 避免在索引列上使用函数

```sql
-- 不好：索引失效
SELECT * FROM users WHERE DATE(created_at) = '2024-01-01';

-- 好的：使用范围查询
SELECT * FROM users 
WHERE created_at >= '2024-01-01' AND created_at < '2024-01-02';
```

### 4. 最左前缀原则

复合索引需要遵循最左前缀原则：

```sql
CREATE INDEX idx_name_age ON users(name, age);

-- 可以使用索引
SELECT * FROM users WHERE name = 'John';
SELECT * FROM users WHERE name = 'John' AND age = 25;

-- 不能使用索引
SELECT * FROM users WHERE age = 25;
```

### 5. 索引维护成本

- 索引不是越多越好，每个索引都有维护成本
- 写操作频繁的表要谨慎添加索引
- 定期使用 `ANALYZE TABLE` 更新统计信息

## 相关工具

- `EXPLAIN` - 分析查询执行计划
- `SHOW INDEX FROM table_name` - 查看表的索引
- `pt-duplicate-key-checker` - 检查重复索引
