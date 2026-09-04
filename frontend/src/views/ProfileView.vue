<template>
  <div class="page">
    <el-header class="header">
      <span class="brand">企业知识库问答系统</span>
      <div class="nav">
        <el-button text @click="$router.push('/chat')">聊天</el-button>
        <el-button v-if="auth.isAdmin" text @click="$router.push('/kb')">知识库管理</el-button>
        <el-button text @click="handleLogout">退出登录</el-button>
      </div>
    </el-header>

    <el-card style="max-width: 500px; margin: 40px auto">
      <h3>修改密码</h3>
      <el-form label-width="90px">
        <el-form-item label="原密码">
          <el-input v-model="oldPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="新密码">
          <el-input v-model="newPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="确认新密码">
          <el-input v-model="confirm" type="password" show-password />
        </el-form-item>
        <el-button type="primary" :loading="loading" @click="handleChange">确认修改</el-button>
      </el-form>
    </el-card>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import api from '../api'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const auth = useAuthStore()
const oldPassword = ref('')
const newPassword = ref('')
const confirm = ref('')
const loading = ref(false)

async function handleChange() {
  if (newPassword.value !== confirm.value) {
    ElMessage.warning('两次新密码不一致')
    return
  }
  loading.value = true
  try {
    await api.post('/auth/change-password', {
      old_password: oldPassword.value,
      new_password: newPassword.value,
    })
    ElMessage.success('密码修改成功，请重新登录')
    auth.logout()
    router.push('/login')
  } catch (e) {
    // 错误已由拦截器提示
  } finally {
    loading.value = false
  }
}

function handleLogout() {
  auth.logout()
  router.push('/login')
}
</script>

<style scoped>
.page {
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
</style>
