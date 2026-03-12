# RESTful API 设计规范

## 基本原则

### 1. 资源导向

- 使用名词表示资源，不使用动词
- 使用复数形式
- 使用小写字母

```
✅ /api/users
✅ /api/users/123/orders
❌ /api/getUsers
❌ /api/createUser
```

### 2. HTTP 方法语义

| 方法 | 用途 | 幂等性 |
|------|------|--------|
| GET | 获取资源 | 是 |
| POST | 创建资源 | 否 |
| PUT | 更新资源（全量） | 是 |
| PATCH | 更新资源（部分） | 是 |
| DELETE | 删除资源 | 是 |

### 3. 状态码规范

```
200 OK - 成功
201 Created - 创建成功
204 No Content - 删除成功

400 Bad Request - 请求参数错误
401 Unauthorized - 未授权
403 Forbidden - 禁止访问
404 Not Found - 资源不存在
409 Conflict - 资源冲突
422 Unprocessable Entity - 参数验证失败

500 Internal Server Error - 服务器错误
```

## 响应格式

### 成功响应

```json
{
  "data": {
    "id": 123,
    "name": "John",
    "email": "john@example.com"
  },
  "meta": {
    "request_id": "abc123"
  }
}
```

### 错误响应

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "参数验证失败",
    "details": [
      {
        "field": "email",
        "message": "邮箱格式不正确"
      }
    ]
  },
  "meta": {
    "request_id": "abc123"
  }
}
```

## 版本控制

```
✅ /api/v1/users
✅ /api/v2/users

# 或使用 Header
Accept: application/vnd.myapi.v1+json
```

## 分页

```
GET /api/users?page=1&per_page=20

响应：
{
  "data": [...],
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 100,
    "total_pages": 5
  }
}
```
