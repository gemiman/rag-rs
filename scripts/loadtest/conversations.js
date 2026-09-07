// 场景②：会话管理（100 并发）
// 目标：测 SQLite 并发读写，暴露 database is locked 问题
import http from 'k6/http';
import { check, sleep } from 'k6';
import { BASE_URL, TEST_PASSWORD, randomUsername, login, register, authHeaders } from './common.js';

export const options = {
  stages: [
    { duration: '20s', target: 10 },
    { duration: '30s', target: 50 },
    { duration: '30s', target: 100 },
    { duration: '30s', target: 100 },
    { duration: '10s', target: 0 },
  ],
  thresholds: {
    http_req_failed: ['rate<0.05'],
  },
};

export default function () {
  // 每个虚拟用户先注册并登录，拿到自己的 token
  const username = randomUsername();
  register(username, TEST_PASSWORD);
  const token = login(username, TEST_PASSWORD);
  if (!token) return;

  // 建会话
  const createRes = http.post(
    `${BASE_URL}/api/conversations`,
    JSON.stringify({ title: '压测会话' }),
    { headers: authHeaders(token) },
  );
  check(createRes, { '建会话成功(200)': (r) => r.status === 200 });
  const convId = createRes.status === 200 ? createRes.json().id : null;

  // 拉会话列表（读）
  const listRes = http.get(`${BASE_URL}/api/conversations`, { headers: authHeaders(token) });
  check(listRes, { '会话列表成功(200)': (r) => r.status === 200 });

  // 查历史消息（读）
  if (convId) {
    const msgsRes = http.get(`${BASE_URL}/api/conversations/${convId}/messages`, { headers: authHeaders(token) });
    check(msgsRes, { '查历史成功(200)': (r) => r.status === 200 });
  }

  sleep(1);
}
