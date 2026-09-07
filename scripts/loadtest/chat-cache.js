// 场景③：问答·缓存命中（100 并发）
// 目标：用固定问题故意命中问答缓存（绕过百炼大模型），
//       测高并发下「Redis 缓存 + SSE 流式回放 + 并发写 messages 表」能不能扛住
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

// 固定问题：第一次问会真调一次大模型并缓存，之后 1 小时内都命中缓存
const FIXED_QUESTION = '智能手机X100的电池容量是多少？';

export default function () {
  const username = randomUsername();
  register(username, TEST_PASSWORD);
  const token = login(username, TEST_PASSWORD);
  if (!token) return;

  const createRes = http.post(
    `${BASE_URL}/api/conversations`,
    JSON.stringify({ title: '压测会话' }),
    { headers: authHeaders(token) },
  );
  const convId = createRes.status === 200 ? createRes.json().id : null;
  if (!convId) return;

  // 固定问题 → 命中缓存路径（SSE 回放）
  const chatRes = http.post(
    `${BASE_URL}/api/chat`,
    JSON.stringify({ conversation_id: convId, question: FIXED_QUESTION }),
    { headers: authHeaders(token), timeout: '120s' },
  );
  check(chatRes, {
    '问答成功(200)': (r) => r.status === 200,
    '收到完成事件(done)': (r) => r.body && r.body.includes('"done"'),
  });

  sleep(1);
}
