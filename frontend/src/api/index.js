import axios from 'axios'
import { ElMessage } from 'element-plus'
import { useAuthStore } from '../stores/auth'
import router from '../router'

const api = axios.create({
  baseURL: '/api',
  timeout: 120000,
})

// 请求拦截：自动带上 JWT
api.interceptors.request.use((config) => {
  const auth = useAuthStore()
  if (auth.token) {
    config.headers.Authorization = `Bearer ${auth.token}`
  }
  return config
})

// 响应拦截：统一错误提示
api.interceptors.response.use(
  (res) => res.data,
  (err) => {
    const status = err.response?.status
    const detail = err.response?.data?.detail || '请求失败'
    if (status === 401) {
      const auth = useAuthStore()
      auth.logout()
      router.push('/login')
    } else {
      ElMessage.error(detail)
    }
    return Promise.reject(err)
  }
)

export default api
