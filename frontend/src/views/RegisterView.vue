<template>
  <div class="auth-page">
    <el-card class="auth-card">
      <h2 class="title">注册账号</h2>
      <el-form @submit.prevent="handleRegister">
        <el-form-item>
          <el-input v-model="username" placeholder="用户名（至少 3 个字符）" size="large" />
        </el-form-item>
        <el-form-item>
          <el-input
            v-model="password"
            type="password"
            placeholder="密码（至少 6 个字符）"
            size="large"
            show-password
          />
        </el-form-item>
        <el-form-item>
          <el-input
            v-model="confirm"
            type="password"
            placeholder="确认密码"
            size="large"
            show-password
            @keyup.enter="handleRegister"
          />
        </el-form-item>
        <el-button
          type="primary"
          size="large"
          style="width: 100%"
          :loading="loading"
          @click="handleRegister"
        >
          注册
        </el-button>
      </el-form>
      <div class="links">
        <el-link type="primary" @click="$router.push('/login')">已有账号？去登录</el-link>
      </div>
    </el-card>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import api from '../api'

const router = useRouter()
const username = ref('')
const password = ref('')
const confirm = ref('')
const loading = ref(false)

async function handleRegister() {
  if (!username.value || !password.value) {
    ElMessage.warning('请填写完整')
    return
  }
  if (password.value !== confirm.value) {
    ElMessage.warning('两次密码不一致')
    return
  }
  loading.value = true
  try {
    await api.post('/auth/register', {
      username: username.value,
      password: password.value,
    })
    ElMessage.success('注册成功，请登录')
    router.push('/login')
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
  margin-bottom: 24px;
}
.links {
  text-align: center;
  margin-top: 16px;
}
</style>
