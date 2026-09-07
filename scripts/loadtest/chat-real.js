// 场景④：真实问答链路（10~20 小并发）
// 目标：用「唯一问题」逼真走「向量化 → 检索 → 百炼大模型 → 流式」全链路，
//       验证外部 API 在并发下是否会超时/报错。⚠️ 会真实调用百炼，烧钱，控制总量。
import http from 'k6/http';
import { check, sleep } from 'k6';
import { BASE_URL, TEST_PASSWORD, randomUsername, login, register, authHeaders, uniqueQuestion } from './common.js';

export const options = {
  stages: [
    { duration: '10s', target: 5 },    // 5 并发预热
    { duration: '20s', target: 15 },   // 加到 15（真实链路，控制并发别烧钱）
    { duration: '30s', target: 15 },   // 保持 15
    { duration: '10s', target: 0 },    // 收尾
  ],
  thresholds: {
    // 真实链路允许更高错误率（百炼可能限流/超时）
    http_req_failed: ['rate<0.20'],
  },
};

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

  // 唯一问题（带随机后缀，避免命中缓存，真走百炼）
  const question = uniqueQuestion('智能手机X100的屏幕尺寸是多少');
  const chatRes = http.post(
    `${BASE_URL}/api/chat`,
    JSON.stringify({ conversation_id: convId, question }),
    { headers: authHeaders(token), timeout: '180s' },
  );
  check(chatRes, {
    '真实问答成功(200)': (r) => r.status === 200,
    '收到完成事件(done)': (r) => r.body && r.body.includes('"done"'),
  });

  sleep(2);
}
