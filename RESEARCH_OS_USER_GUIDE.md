# Research OS 使用指南

## 启动应用

Desktop应用已成功启动！你现在可以看到完整的Research OS功能。

---

## 功能验证清单

### 1. 检查右侧导航栏
**位置:** 应用右侧  
**预期效果:**
- ✅ 看到16个Research Domain图标
- ✅ 图标无重复
- ✅ 每个图标右下角有小圆点状态徽章：
  - 🟢 绿色 = 所有native actions就绪
  - 🟡 黄色 = 部分actions就绪  
  - 🔴 红色 = SDK缺失
- ✅ 鼠标悬停显示详细信息（如 "2/3 actions ready"）

### 2. 打开Research OS面板
**操作:** 点击右下角的圆形浮动按钮（带有竖条图标）  
**预期效果:**
- ✅ 从右侧滑出420px宽的面板
- ✅ 面板顶部显示 "Research OS" 标题
- ✅ 看到6个标签：Hypotheses, Evidence, Experiments, Negative Results, Diary, Timeline
- ✅ 默认显示 "Hypotheses" 标签

### 3. 测试每个标签页
**操作:** 依次点击每个标签  
**预期效果:**

#### Hypotheses 标签
- 如果是首次使用，显示 "No hypotheses yet"
- 如果有数据，显示：
  - 标题
  - 状态徽章（draft/active/validated/refuted）
  - 描述文本
  - Domain标签
  - 创建时间
  - Evidence数量

#### Evidence 标签
- 显示所有证据条目
- ✓ 图标表示支持，✗ 图标表示反驳
- 星级显示强度（0-5星）
- 证据类型徽章（experimental/literature/artifact/benchmark）

#### Experiments 标签
- 显示实验列表
- 状态徽章（planned/running/completed/failed）
- Artifacts计数
- 关联的domain

#### Negative Results 标签
- 显示失败记录
- 失败模式标签
- "Learned" 部分高亮显示教训
- 相似度警告（如果存在类似失败）

#### Diary 标签
- 按时间倒序显示日记条目
- 类型徽章（observation/decision/question/insight）
- 作者信息（user/agent）
- 时间戳

#### Timeline 标签
- 按时间正序显示事件
- 时间线视觉样式（左侧有竖线）
- 事件类型徽章
- 事件描述

### 4. 测试自动摄取功能

**步骤:**
1. 在应用中打开任意Research Domain（如 ai-ml）
2. 运行一个Domain Task（点击任意Native Action）
3. 等待任务完成
4. 打开Research OS面板
5. 检查以下标签：
   - **Experiments** - 应该看到新的实验记录
   - **Diary** - 应该看到新的日记条目
   - **Timeline** - 应该看到新的时间线事件
   - **Evidence** - 如果任务生成了artifacts，应该有新的证据

**如果任务失败:**
- **Negative Results** 标签应该有新记录
- 应该显示失败原因和学到的教训

### 5. 测试主题切换
**操作:** 切换深色/浅色主题  
**预期效果:**
- ✅ Research OS面板颜色正确适配
- ✅ 状态徽章可读性良好
- ✅ 浮动按钮可见且美观

---

## 已知的初始状态

由于这是首次启动，Research OS可能是空的。要填充数据：

### 方法1: 运行Domain Tasks
1. 选择一个domain（推荐：software-engineering 或 distributed-systems，因为它们有ready的actions）
2. 运行任意task
3. Research OS会自动记录

### 方法2: 使用Agent
1. 在chat中让agent执行research相关任务
2. Agent的操作会自动写入Research OS

### 方法3: 直接通过API创建（高级）
```bash
# 创建hypothesis示例
curl -X POST http://localhost:PORT/api/research-os/hypothesis \
  -H "Content-Type: application/json" \
  -d '{
    "title": "测试假设",
    "description": "这是一个测试假设",
    "domain_id": "ai-ml"
  }'
```

---

## 故障排查

### 问题1: 看不到Research OS按钮
**检查:**
- 浏览器控制台是否有JavaScript错误
- `frontend/research-os.js` 是否正确加载
- 页面是否完全加载

**解决:** 刷新页面 (Ctrl+R)

### 问题2: 点击标签页没有数据
**原因:** 这是正常的，表示还没有对应类型的对象  
**解决:** 运行一些domain tasks来生成数据

### 问题3: 状态徽章不显示
**检查:**
- 是否所有domain都没有native_actions
- CSS是否正确加载

**解决:** 检查浏览器开发者工具中的CSS加载情况

### 问题4: API返回错误
**检查后端日志:**
```bash
# 查看控制台输出
# 应该看到API请求日志
```

**常见原因:**
- 工作区路径错误
- JSON序列化错误
- 文件系统权限

---

## API端点测试

你可以直接测试API端点（假设应用在localhost:3000）：

```bash
# 列出所有hypotheses
curl http://localhost:3000/api/research-os/hypotheses

# 列出所有evidence
curl http://localhost:3000/api/research-os/evidence

# 列出所有experiments
curl http://localhost:3000/api/research-os/experiments

# 列出所有negative results
curl http://localhost:3000/api/research-os/negative-results

# 列出所有diary entries
curl http://localhost:3000/api/research-os/diary

# 列出所有timeline events
curl http://localhost:3000/api/research-os/timeline

# 列出所有publications
curl http://localhost:3000/api/research-os/publications
```

**预期响应格式:**
```json
{
  "ok": true,
  "data": {
    "hypotheses": [],
    "evidence": [],
    ...
  }
}
```

---

## 数据持久化验证

Research OS数据存储在：
```
D:\Atlas\.atlas\research-os\
├── hypothesis\
├── evidence\
├── experiment\
├── negative-result\
├── diary\
├── kg-node\
├── kg-edge\
├── decision\
├── memory\
├── timeline\
└── publication\
```

**检查方法:**
```bash
# 查看已创建的对象
ls -la D:/Atlas/.atlas/research-os/*/

# 查看某个对象的内容
cat D:/Atlas/.atlas/research-os/diary/[id].json
```

---

## 性能验证

### 预期性能指标
- API响应时间: < 100ms（空数据）
- API响应时间: < 500ms（100个对象）
- UI渲染时间: < 200ms
- 面板滑出动画: 流畅（无卡顿）

### 压力测试
如果需要测试大量数据：
1. 运行多个domain tasks
2. 检查面板渲染性能
3. 验证滚动流畅度

---

## 成功标准

### ✅ 基础功能
- [x] Desktop应用启动成功
- [ ] Research OS按钮可见
- [ ] 面板可以打开/关闭
- [ ] 所有6个标签可以切换
- [ ] API端点返回正确格式

### ✅ 导航增强
- [ ] 16个domain图标无重复
- [ ] 状态徽章正确显示
- [ ] 徽章颜色准确反映状态

### ✅ 自动摄取
- [ ] 运行task后生成experiment
- [ ] 生成对应的diary entry
- [ ] 生成timeline event
- [ ] 失败任务生成negative result

### ✅ 用户体验
- [ ] 面板动画流畅
- [ ] 深色/浅色主题都正常
- [ ] 响应式布局工作正常
- [ ] 无JavaScript错误

---

## 下一步行动

1. **立即验证:**
   - 打开应用，检查UI
   - 点击Research OS按钮
   - 浏览所有标签页
   - 运行一个domain task测试自动摄取

2. **数据填充:**
   - 运行几个不同的domain tasks
   - 在多个domains中创建数据
   - 观察Research OS的增长

3. **功能探索:**
   - 测试失败场景（创建会失败的task）
   - 观察negative results的记录
   - 检查相似失败检测是否工作

4. **性能评估:**
   - 创建大量对象（>50个）
   - 测试面板响应性
   - 评估加载时间

---

## 技术支持

如果遇到问题：
1. 查看浏览器开发者工具控制台
2. 检查后端日志输出
3. 验证 `.atlas/research-os/` 目录权限
4. 确认所有文件正确保存

**关键文件位置:**
- Backend: `src/research_os/`
- Frontend: `frontend/research-os.js`
- Styles: `frontend/styles.css`
- API: `src/web.rs` (lines ~32260-32340)

---

## 祝贺！🎉

你现在拥有了一个完整的Research OS系统，可以：
- 自动记录研究过程
- 追踪假设和证据
- 管理实验血缘
- 从失败中学习
- 维护完整的科学时间线

开始你的研究之旅吧！
