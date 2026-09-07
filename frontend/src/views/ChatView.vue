<template>
  <div class="chat-page">
    <!-- 左侧：会话列表 -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <el-button type="primary" style="width: 100%" @click="newConversation">+ 新建对话</el-button>
      </div>
      <el-scrollbar class="conv-list">
        <div
          v-for="c in conversations"
          :key="c.id"
          class="conv-item"
          :class="{ active: c.id === currentId }"
          @click="selectConversation(c)"
        >
          <div class="conv-title">{{ c.title }}</div>
          <div class="conv-actions">
            <el-icon title="重命名" @click.stop="renameConversation(c)"><Edit /></el-icon>
            <el-icon title="删除" @click.stop="deleteConversation(c)"><Delete /></el-icon>
          </div>
        </div>
        <div v-if="!conversations.length" class="empty">暂无会话，点击上方新建</div>
      </el-scrollbar>
      <div class="sidebar-footer">
        <span class="user-name">{{ auth.user?.username }}</span>
        <el-button text @click="$router.push('/profile')">个人中心</el-button>
        <el-button v-if="auth.isAdmin" text @click="$router.push('/kb')">知识库管理</el-button>
        <el-button text @click="logout">退出</el-button>
      </div>
    </aside>

    <!-- 右侧：聊天区 -->
    <main class="main">
      <div class="messages" ref="messagesRef">
        <el-empty v-if="!messages.length && !streaming" description="开始提问吧" />
        <MessageBubble v-for="m in messages" :key="m.id" :message="m" />
      </div>
      <div class="input-area">
        <el-input
          v-model="question"
          type="textarea"
          :rows="3"
          placeholder="输入问题，回车发送（Shift+回车换行）"
          @keydown.enter.exact.prevent="send"
        />
        <el-button
          type="primary"
          class="send-btn"
          :loading="streaming"
          :disabled="!question.trim()"
          @click="send"
        >
          发送
        </el-button>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, reactive, nextTick, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import api from '../api'
import { useAuthStore } from '../stores/auth'
import MessageBubble from '../components/MessageBubble.vue'

const router = useRouter()
const auth = useAuthStore()

const conversations = ref([])
const currentId = ref(null)
const messages = ref([])
const question = ref('')
const streaming = ref(false)
const messagesRef = ref(null)

onMounted(() => {
  fetchConversations()
})

async function fetchConversations() {
  const data = await api.get('/conversations')
  conversations.value = data
  if (data.length && !currentId.value) {
    selectConversation(data[0])
  }
}

async function newConversation() {
  const data = await api.post('/conversations', { title: '新对话' })
  conversations.value.unshift(data)
  currentId.value = data.id
  messages.value = []
}

async function selectConversation(c) {
  currentId.value = c.id
  const data = await api.get(`/conversations/${c.id}/messages`)
  messages.value = data
  scrollToBottom()
}

async function renameConversation(c) {
  const { value } = await ElMessageBox.prompt('请输入新的会话标题', '重命名', {
    inputValue: c.title,
  })
  await api.put(`/conversations/${c.id}`, { title: value })
  c.title = value
}

async function deleteConversation(c) {
  await ElMessageBox.confirm('确定删除该会话吗？', '提示', { type: 'warning' })
  await api.delete(`/conversations/${c.id}`)
  conversations.value = conversations.value.filter((x) => x.id !== c.id)
  if (currentId.value === c.id) {
    currentId.value = null
    messages.value = []
    if (conversations.value.length) selectConversation(conversations.value[0])
  }
}

async function send() {
  const q = question.value.trim()
  if (!q || streaming.value) return
  if (!currentId.value) await newConversation()

  question.value = ''
  messages.value.push({ id: Date.now(), role: 'user', content: q, citations: [] })
  // 用 reactive 包裹，保证流式追加内容时界面能自动刷新
  const assistantMsg = reactive({ id: Date.now() + 1, role: 'assistant', content: '', citations: [] })
  messages.value.push(assistantMsg)
  streaming.value = true
  scrollToBottom()

  try {
    await streamChat(currentId.value, q, {
      onDelta: (text) => {
        assistantMsg.content += text
        scrollToBottom()
      },
      onDone: (citations) => {
        assistantMsg.citations = citations || []
      },
      onError: (msg) => {
        assistantMsg.content = '出错了：' + msg
      },
    })
  } catch (e) {
    assistantMsg.content = '请求失败，请重试'
  } finally {
    streaming.value = false
    scrollToBottom()
    fetchConversations()
  }
}

// 通过 fetch 流式读取 SSE 响应
async function streamChat(conversationId, questionText, { onDelta, onDone, onError }) {
  const res = await fetch('/api/chat', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${auth.token}`,
    },
    body: JSON.stringify({ conversation_id: conversationId, question: questionText }),
  })
  if (!res.ok) {
    const data = await res.json().catch(() => ({}))
    throw new Error(data.detail || '请求失败')
  }

  const reader = res.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    const lines = buffer.split('\n')
    buffer = lines.pop() || ''
    for (const line of lines) {
      if (line.startsWith('data:')) {
        const payload = line.slice(5).trim()
        if (!payload) continue
        try {
          const msg = JSON.parse(payload)
          if (msg.type === 'delta') onDelta(msg.text)
          else if (msg.type === 'done') onDone(msg.citations)
          else if (msg.type === 'error') onError(msg.message)
        } catch (e) {
          // 忽略无法解析的行
        }
      }
    }
  }
}

function scrollToBottom() {
  nextTick(() => {
    if (messagesRef.value) {
      messagesRef.value.scrollTop = messagesRef.value.scrollHeight
    }
  })
}

function logout() {
  auth.logout()
  router.push('/login')
}
</script>

<style scoped>
.chat-page {
  display: flex;
  height: 100vh;
}
.sidebar {
  width: 260px;
  display: flex;
  flex-direction: column;
  background: #fff;
  border-right: 1px solid #e4e7ed;
}
.sidebar-header {
  padding: 12px;
}
.conv-list {
  flex: 1;
}
.conv-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  cursor: pointer;
  border-bottom: 1px solid #f0f0f0;
}
.conv-item:hover {
  background: #f5f7fa;
}
.conv-item.active {
  background: #ecf5ff;
}
.conv-title {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 14px;
}
.conv-actions {
  display: none;
  gap: 6px;
}
.conv-item:hover .conv-actions {
  display: flex;
}
.empty {
  text-align: center;
  color: #bbb;
  padding: 20px;
  font-size: 13px;
}
.sidebar-footer {
  padding: 10px;
  border-top: 1px solid #e4e7ed;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
}
.user-name {
  margin-right: auto;
  font-size: 13px;
  color: #666;
}
.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: #f5f7fa;
  padding: 0;
}
.messages {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}
.input-area {
  display: flex;
  gap: 10px;
  padding: 16px;
  background: #fff;
  border-top: 1px solid #e4e7ed;
  align-items: flex-end;
}
.send-btn {
  height: 56px;
}
</style>
