import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = {
  vus: 10,
  duration: '60s',
  thresholds: {
    http_req_duration: ['p(95)<500'],
    http_req_failed: ['rate<0.1'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://hodei-server:8080';

export default function() {
  let healthResponse = http.get(`${BASE_URL}/health`);
  check(healthResponse, {
    'health status is 200': (r) => r.status === 200,
    'health response time OK': (r) => r.timings.duration < 500,
  });

  let jobsResponse = http.get(`${BASE_URL}/api/v1/jobs`);
  check(jobsResponse, {
    'jobs status is 200 or 401': (r) => r.status === 200 || r.status === 401,
    'jobs response time OK': (r) => r.timings.duration < 1000,
  });

  let metricsResponse = http.get(`${BASE_URL}:9091/metrics`);
  check(metricsResponse, {
    'metrics status is 200': (r) => r.status === 200,
  });

  sleep(1);
}
