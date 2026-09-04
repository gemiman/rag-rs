<template>
  <div class="kb-page">
    <el-header class="header">
      <span class="brand">知识库管理</span>
      <div class="nav">
        <el-button text @click="$router.push('/chat')">聊天</el-button>
        <el-button text @click="$router.push('/profile')">个人中心</el-button>
        <el-button text @click="logout">退出</el-button>
      </div>
    </el-header>

    <el-main>
      <el-card>
        <div class="upload-section">
          <el-upload
            :http-request="customUpload"
            :show-file-list="false"
            accept=".txt,.md,.csv,.xlsx,.xls,.pdf,.docx"
          >
            <el-button type="primary" :icon="'Upload'">上传文档</el-button>
          </el-upload>
          <span class="hint">支持 txt / md / csv / xlsx / pdf / docx，上传后自动解析入库</span>
        </div>
      </el-card>

      <el-card style="margin-top: 16px">
        <el-table :data="documents" v-loading="loading">
          <el-table-column prop="id" label="ID" width="60" />
          <el-table-column prop="filename" label="文件名" min-width="200" />
          <el-table-column prop="file_type" label="类型" width="80" />
          <el-table-column label="大小" width="100">
            <template #default="{ row }">{{ formatSize(row.size) }}</template>
          </el-table-column>
          <el-table-column prop="chunk_count" label="片段数" width="90" />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag v-if="row.status === 'done'" type="success">已完成</el-tag>
              <el-tag v-else-if="row.status === 'processing'" type="warning">处理中</el-tag>
              <el-tag v-else type="danger">失败</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="上传时间" width="180">
            <template #default="{ row }">{{ formatTime(row.created_at) }}</template>
          </el-table-column>
          <el-table-column label="操作" width="180">
            <template #default="{ row }">
              <el-button size="small" :disabled="row.status !== 'done'" @click="preview(row)">
                预览片段
              </el-button>
              <el-button size="small" type="danger" @click="remove(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </el-main>

    <el-dialog v-model="previewVisible" title="片段预览" width="700px">
      <el-scrollbar max-height="60vh">
        <div v-for="c in chunks" :key="c.index" class="chunk">
          <div class="chunk-index">片段 {{ c.index + 1 }}</div>
          <div class="chunk-content">{{ c.content }}</div>
        </div>
        <el-empty v-if="!chunks.length" description="暂无片段" />
      </el-scrollbar>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import api from '../api'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const auth = useAuthStore()
const documents = ref([])
const loading = ref(false)
const previewVisible = ref(false)
const chunks = ref([])
let timer = null

onMounted(() => {
  fetchDocuments()
  // 每 3 秒刷新一次，自动更新"处理中"状态
  timer = setInterval(fetchDocuments, 3000)
})
onUnmounted(() => clearInterval(timer))

async function fetchDocuments() {
  loading.value = true
  try {
    documents.value = await api.get('/kb/documents')
  } finally {
    loading.value = false
  }
}

async function customUpload({ file }) {
  const formData = new FormData()
  formData.append('file', file)
  await api.post('/kb/documents', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  })
  ElMessage.success('上传成功，正在后台解析入库')
  fetchDocuments()
}

async function remove(row) {
  await ElMessageBox.confirm(`确定删除「${row.filename}」吗？`, '提示', { type: 'warning' })
  await api.delete(`/kb/documents/${row.id}`)
  ElMessage.success('已删除')
  fetchDocuments()
}

async function preview(row) {
  chunks.value = await api.get(`/kb/documents/${row.id}/chunks`)
  previewVisible.value = true
}

function formatSize(bytes) {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1024 / 1024).toFixed(1) + ' MB'
}

function formatTime(s) {
  if (!s) return ''
  return s.replace('T', ' ').slice(0, 19)
}

function logout() {
  auth.logout()
  router.push('/login')
}
</script>

<style scoped>
.kb-page {
  min-height: 100vh;
  background: #f5f7fa;
}
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #fff;
  border-bottom: 1px solid #e4e7ed;
}
.brand {
  font-weight: 600;
}
.upload-section {
  display: flex;
  align-items: center;
  gap: 16px;
}
.hint {
  color: #999;
  font-size: 13px;
}
.chunk {
  margin-bottom: 14px;
  border-bottom: 1px dashed #eee;
  padding-bottom: 10px;
}
.chunk-index {
  color: #409eff;
  font-size: 13px;
  margin-bottom: 4px;
}
.chunk-content {
  color: #606266;
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
}
</style>
