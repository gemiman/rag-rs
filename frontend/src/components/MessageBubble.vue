<template>
  <div class="msg" :class="message.role">
    <div class="avatar">{{ message.role === 'user' ? '我' : 'AI' }}</div>
    <div class="bubble">
      <div class="content">{{ message.content || '...' }}</div>
      <div v-if="message.citations && message.citations.length" class="citations">
        <div class="citations-title">📚 引用知识库片段：</div>
        <div v-for="c in message.citations" :key="c.index" class="citation-item">
          <div class="citation-source">[{{ c.index }}] 来源：{{ c.source }}</div>
          <div class="citation-content">{{ c.content }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
defineProps({
  message: {
    type: Object,
    required: true,
  },
})
</script>

<style scoped>
.msg {
  display: flex;
  margin-bottom: 16px;
}
.msg.user {
  flex-direction: row-reverse;
}
.avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 13px;
  flex-shrink: 0;
}
.msg.user .avatar {
  background: #409eff;
}
.msg.assistant .avatar {
  background: #67c23a;
}
.bubble {
  max-width: 72%;
  padding: 10px 14px;
  border-radius: 10px;
  margin: 0 10px;
  line-height: 1.6;
}
.msg.user .bubble {
  background: #409eff;
  color: #fff;
  border-top-right-radius: 2px;
}
.msg.assistant .bubble {
  background: #fff;
  border: 1px solid #e4e7ed;
  border-top-left-radius: 2px;
}
.content {
  white-space: pre-wrap;
  word-break: break-word;
}
.citations {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed #dcdfe6;
}
.citations-title {
  font-size: 12px;
  color: #909399;
  margin-bottom: 6px;
}
.citation-item {
  background: #f5f7fa;
  border-radius: 6px;
  padding: 6px 8px;
  margin-bottom: 6px;
  font-size: 12px;
}
.citation-source {
  color: #409eff;
  margin-bottom: 2px;
}
.citation-content {
  color: #606266;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
