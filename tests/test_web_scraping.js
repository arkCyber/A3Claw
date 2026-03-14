#!/usr/bin/env node
/**
 * OpenClaw 网页信息搜集功能测试
 * 测试网络请求、RSS 解析、数据提取等功能
 */

import * as net from 'wasi_net';

// 测试配置
const TEST_CASES = [
  {
    name: 'NPR News RSS Feed',
    host: 'feeds.npr.org',
    path: '/1001/rss.xml',
    expectedKeywords: ['news', 'title', 'description']
  },
  {
    name: 'CBS News RSS Feed',
    host: 'www.cbsnews.com',
    path: '/latest/rss/main',
    expectedKeywords: ['item', 'title', 'link']
  }
];

// HTTP GET 请求函数
async function httpsGet(host, path) {
  const reqText = `GET ${path} HTTP/1.1\r\n` +
    `Host: ${host}\r\n` +
    `User-Agent: OpenClaw-Test/1.0\r\n` +
    `Accept: application/rss+xml, application/xml, text/xml, */*\r\n` +
    `Connection: close\r\n\r\n`;
  
  const enc = new Uint8Array(reqText.length);
  for (let i = 0; i < reqText.length; i++) {
    enc[i] = reqText.charCodeAt(i) & 0xff;
  }

  print(`[TEST] Connecting to ${host}:443`);
  const conn = await net.WasiTlsConn.connect(host, 443);
  await conn.write(enc.buffer);

  const chunks = [];
  let total = 0;
  while (total < 2 * 1024 * 1024) {
    const chunk = await conn.read();
    if (!chunk || chunk.byteLength === 0) break;
    chunks.push(new Uint8Array(chunk));
    total += chunk.byteLength;
  }

  const all = new Uint8Array(total);
  let off = 0;
  for (let i = 0; i < chunks.length; i++) {
    all.set(chunks[i], off);
    off += chunks[i].length;
  }

  let text = '';
  for (let i = 0; i < all.length; i++) {
    text += String.fromCharCode(all[i]);
  }

  return text;
}

// 解析 HTTP 响应
function parseHttpResponse(response) {
  const headerEnd = response.indexOf('\r\n\r\n');
  if (headerEnd === -1) return null;
  
  const headers = response.substring(0, headerEnd);
  const body = response.substring(headerEnd + 4);
  
  const statusLine = headers.split('\r\n')[0];
  const statusMatch = statusLine.match(/HTTP\/\d\.\d (\d+)/);
  const statusCode = statusMatch ? parseInt(statusMatch[1]) : 0;
  
  return { statusCode, headers, body };
}

// 提取 RSS 项目
function extractRssItems(xml) {
  const items = [];
  const itemRegex = /<item>([\s\S]*?)<\/item>/g;
  let match;
  
  while ((match = itemRegex.exec(xml)) !== null) {
    const itemXml = match[1];
    const titleMatch = itemXml.match(/<title>(.*?)<\/title>/);
    const linkMatch = itemXml.match(/<link>(.*?)<\/link>/);
    const descMatch = itemXml.match(/<description>(.*?)<\/description>/);
    
    if (titleMatch) {
      items.push({
        title: titleMatch[1],
        link: linkMatch ? linkMatch[1] : '',
        description: descMatch ? descMatch[1].substring(0, 100) : ''
      });
    }
  }
  
  return items;
}

// 运行测试
async function runTests() {
  print('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  print('  OpenClaw 网页信息搜集功能测试');
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  const results = [];
  
  for (const testCase of TEST_CASES) {
    print(`\n[TEST] ${testCase.name}`);
    print(`[INFO] 请求: https://${testCase.host}${testCase.path}`);
    
    try {
      const startTime = Date.now();
      const response = await httpsGet(testCase.host, testCase.path);
      const elapsed = Date.now() - startTime;
      
      const parsed = parseHttpResponse(response);
      
      if (!parsed) {
        print(`[FAIL] 无法解析 HTTP 响应`);
        results.push({ name: testCase.name, status: 'FAIL', error: '解析失败' });
        continue;
      }
      
      print(`[INFO] HTTP 状态码: ${parsed.statusCode}`);
      print(`[INFO] 响应时间: ${elapsed}ms`);
      
      if (parsed.statusCode !== 200) {
        print(`[FAIL] HTTP 状态码错误: ${parsed.statusCode}`);
        results.push({ name: testCase.name, status: 'FAIL', error: `状态码 ${parsed.statusCode}` });
        continue;
      }
      
      // 检查关键字
      let keywordsFound = 0;
      for (const keyword of testCase.expectedKeywords) {
        if (parsed.body.toLowerCase().includes(keyword.toLowerCase())) {
          keywordsFound++;
        }
      }
      
      print(`[INFO] 关键字匹配: ${keywordsFound}/${testCase.expectedKeywords.length}`);
      
      // 提取 RSS 项目
      const items = extractRssItems(parsed.body);
      print(`[INFO] 提取到 ${items.length} 条新闻`);
      
      if (items.length > 0) {
        print(`[INFO] 示例新闻: ${items[0].title.substring(0, 60)}...`);
      }
      
      if (keywordsFound >= testCase.expectedKeywords.length && items.length > 0) {
        print(`[PASS] ✅ 测试通过`);
        results.push({ 
          name: testCase.name, 
          status: 'PASS', 
          items: items.length,
          time: elapsed
        });
      } else {
        print(`[FAIL] ❌ 测试失败`);
        results.push({ name: testCase.name, status: 'FAIL', error: '数据验证失败' });
      }
      
    } catch (error) {
      print(`[FAIL] ❌ 异常: ${error.message || error}`);
      results.push({ name: testCase.name, status: 'FAIL', error: error.message || String(error) });
    }
  }
  
  // 生成测试报告
  print('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  print('  测试结果汇总');
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  let passed = 0;
  let failed = 0;
  
  for (const result of results) {
    if (result.status === 'PASS') {
      print(`✅ ${result.name} - 通过 (${result.items} 条数据, ${result.time}ms)`);
      passed++;
    } else {
      print(`❌ ${result.name} - 失败: ${result.error}`);
      failed++;
    }
  }
  
  print(`\n总计: ${passed} 通过, ${failed} 失败`);
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  return { passed, failed, results };
}

// 执行测试
runTests().then(result => {
  print('[INFO] 网页信息搜集测试完成');
}).catch(error => {
  print(`[ERROR] 测试执行失败: ${error}`);
});
