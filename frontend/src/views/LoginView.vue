<template>
  <div class="auth-page">
    <el-card class="auth-card">
      <h2 class="title">企业知识库问答系统</h2>
      <p class="subtitle">电商商品知识库 · RAG 智能问答</p>
      <el-form @submit.prevent="handleLogin">
        <el-form-item>
          <el-input v-model="username" placeholder="用户名" size="large" :prefix-icon="'User'" />
        </el-form-item>
        <el-form-item>
          <el-input
            v-model="password"
            type="password"
            placeholder="密码"
            size="large"
            show-password
            :prefix-icon="'Lock'"
            @keyup.enter="handleLogin"
          />
        </el-form-item>
        <el-button
          type="primary"
          size="large"
          style="width: 100%"
          :loading="loading"
          @click="handleLogin"
        >
          登录
        </el-button>
      </el-form>
      <div class="links">
        <el-link type="primary" @click="$router.push('/register')">没有账号？去注册</el-link>
      </div>
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
const username = ref('')
const password = ref('')
const loading = ref(false)

async function handleLogin() {
  if (!username.value || !password.value) {
    ElMessage.warning('请输入用户名和密码')
    return
  }
  loading.value = true
  try {
    const data = await api.post('/auth/login', {
      username: username.value,
      password: password.value,
    })
    auth.setAuth(data.token, data.user)
    ElMessage.success('登录成功')
    router.push('/chat')
  } catch (e) {
    // 错误已由拦截器提示
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.auth-page {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.auth-card {
  width: 380px;
  padding: 20px;
  border-radius: 12px;
}
.title {
  text-align: center;
  margin-bottom: 4px;
}
.subtitle {
  text-align: center;
  color: #999;
  margin-bottom: 24px;
  font-size: 13px;
}
.links {
  text-align: center;
  margin-top: 16px;
}
</style>
