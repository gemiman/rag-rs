// 公共工具：所有压测场景复用
import http from 'k6/http';

// 后端地址（压测直接打后端，不经前端代理）
export const BASE_URL = 'http://127.0.0.1:8000';

// 压测账号统一密码（>=6 位，满足注册规则）
export const TEST_PASSWORD = 'loadtest123';

// 登录，成功返回 token，失败返回 null
export function login(username, password) {
  const res = http.post(
    `${BASE_URL}/api/auth/login`,
    JSON.stringify({ username, password }),
    { headers: { 'Content-Type': 'application/json' } },
  );
  if (res.status === 200) {
    return res.json().token || null;
  }
  return null;
}

// 注册，成功返回 token，失败返回 null（注册接口直接返回 token）
export function register(username, password) {
  const res = http.post(
    `${BASE_URL}/api/auth/register`,
    JSON.stringify({ username, password }),
    { headers: { 'Content-Type': 'application/json' } },
  );
  if (res.status === 200) {
    return res.json().token || null;
  }
  return null;
}

// 生成随机用户名（loadtest 前缀，便于压测后清理）
export function randomUsername() {
  return `loadtest_${Date.now()}_${Math.floor(Math.random() * 1e6)}`;
}

// 组装带 JWT 的请求头
export function authHeaders(token) {
  return {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${token}`,
  };
}

// 生成唯一问题（带随机后缀，避免命中问答缓存，逼真走大模型）
export function uniqueQuestion(prefix) {
  return `${prefix} #${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
}
