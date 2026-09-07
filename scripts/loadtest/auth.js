// 场景①：注册 + 登录（100 并发）
// 目标：测认证接口吞吐，暴露 argon2 密码加密的 CPU 瓶颈
import http from 'k6/http';
import { check, sleep } from 'k6';
import { BASE_URL, TEST_PASSWORD, randomUsername } from './common.js';

export const options = {
  // 从 10 并发逐步加到 100，避免一上来就打崩，方便看拐点
  stages: [
    { duration: '20s', target: 10 },   // 预热：10 并发
    { duration: '30s', target: 50 },   // 加到 50
    { duration: '30s', target: 100 },  // 加到 100（模拟 100 人）
    { duration: '30s', target: 100 },  // 保持 100 打满
    { duration: '10s', target: 0 },    // 收尾
  ],
  thresholds: {
    http_req_failed: ['rate<0.05'], // 错误率超过 5% 判失败
  },
};

export default function () {
  const username = randomUsername();
  const password = TEST_PASSWORD;

  // 注册（argon2 加密密码，CPU 密集）
  const regRes = http.post(
    `${BASE_URL}/api/auth/register`,
    JSON.stringify({ username, password }),
    { headers: { 'Content-Type': 'application/json' } },
  );
  check(regRes, { '注册成功(200)': (r) => r.status === 200 });

  // 登录（argon2 校验密码，CPU 密集）
  const loginRes = http.post(
    `${BASE_URL}/api/auth/login`,
    JSON.stringify({ username, password }),
    { headers: { 'Content-Type': 'application/json' } },
  );
  check(loginRes, { '登录成功(200)': (r) => r.status === 200 });

  sleep(1);
}
