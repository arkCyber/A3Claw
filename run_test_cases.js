// OpenClaw 测试案例运行器
// 在 WasmEdge 环境中测试核心功能

print('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
print('  OpenClaw WasmEdge 功能测试案例');
print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');

const testResults = {
    passed: 0,
    failed: 0,
    tests: []
};

// ============================================================
// 测试案例 1: 文件操作功能
// ============================================================
async function testFileOperations() {
    print('\n[TEST 1] 文件操作功能测试');
    print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    
    try {
        const std = await import('std');
        
        // 测试文件写入
        const testFile = '/workspace/test_openclaw.txt';
        const testContent = 'OpenClaw Test - ' + new Date().toISOString();
        
        print(`\n[1.1] 写入测试文件: ${testFile}`);
        const file = std.open(testFile, 'w');
        if (file) {
            file.puts(testContent + '\n');
            file.puts('WasmEdge 沙箱环境测试\n');
            file.puts('功能: 文件读写操作\n');
            file.close();
            print('  ✅ 文件写入成功');
            testResults.passed++;
            testResults.tests.push({ name: '文件写入', status: 'PASS' });
        } else {
            throw new Error('无法创建文件');
        }
        
        // 测试文件读取
        print(`\n[1.2] 读取测试文件: ${testFile}`);
        const readFile = std.open(testFile, 'r');
        if (readFile) {
            let content = '';
            let line;
            while ((line = readFile.getline()) !== null) {
                content += line + '\n';
            }
            readFile.close();
            
            if (content.includes('OpenClaw Test')) {
                print('  ✅ 文件读取成功');
                print(`  内容长度: ${content.length} 字节`);
                testResults.passed++;
                testResults.tests.push({ name: '文件读取', status: 'PASS' });
            } else {
                throw new Error('文件内容不匹配');
            }
        } else {
            throw new Error('无法读取文件');
        }
        
        return true;
    } catch (error) {
        print(`  ❌ 测试失败: ${error.message || error}`);
        testResults.failed++;
        testResults.tests.push({ name: '文件操作', status: 'FAIL', error: error.message });
        return false;
    }
}

// ============================================================
// 测试案例 2: 网络请求功能
// ============================================================
async function testNetworkRequest() {
    print('\n\n[TEST 2] 网络请求功能测试');
    print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    
    try {
        const net = await import('wasi_net');
        
        const host = 'feeds.npr.org';
        const path = '/1001/rss.xml';
        
        print(`\n[2.1] HTTPS 请求: https://${host}${path}`);
        
        const reqText = `GET ${path} HTTP/1.1\r\nHost: ${host}\r\nUser-Agent: OpenClaw-Test/1.0\r\nConnection: close\r\n\r\n`;
        const enc = new Uint8Array(reqText.length);
        for (let i = 0; i < reqText.length; i++) {
            enc[i] = reqText.charCodeAt(i) & 0xff;
        }
        
        print('  连接中...');
        const conn = await net.WasiTlsConn.connect(host, 443);
        await conn.write(enc.buffer);
        
        print('  接收数据...');
        const chunks = [];
        let total = 0;
        while (total < 1024 * 1024) {
            const chunk = await conn.read();
            if (!chunk || chunk.byteLength === 0) break;
            chunks.push(new Uint8Array(chunk));
            total += chunk.byteLength;
        }
        
        print(`  ✅ 接收完成: ${total} 字节`);
        
        // 解析响应
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
        
        const statusMatch = text.match(/HTTP\/\d\.\d (\d+)/);
        const statusCode = statusMatch ? parseInt(statusMatch[1]) : 0;
        
        print(`  HTTP 状态码: ${statusCode}`);
        
        if (statusCode === 200) {
            print('  ✅ 网络请求成功');
            testResults.passed++;
            testResults.tests.push({ name: '网络请求', status: 'PASS' });
            
            // 提取 RSS 项目数量
            const itemCount = (text.match(/<item>/g) || []).length;
            print(`  RSS 项目数: ${itemCount}`);
            
            return true;
        } else {
            throw new Error(`HTTP 状态码错误: ${statusCode}`);
        }
        
    } catch (error) {
        print(`  ❌ 测试失败: ${error.message || error}`);
        testResults.failed++;
        testResults.tests.push({ name: '网络请求', status: 'FAIL', error: error.message });
        return false;
    }
}

// ============================================================
// 测试案例 3: 数据处理功能
// ============================================================
function testDataProcessing() {
    print('\n\n[TEST 3] 数据处理功能测试');
    print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    
    try {
        // JSON 处理
        print('\n[3.1] JSON 数据处理');
        const testData = {
            name: 'OpenClaw',
            version: '1.0',
            features: ['WasmEdge', 'Security', 'AI'],
            timestamp: Date.now()
        };
        
        const jsonStr = JSON.stringify(testData);
        const parsed = JSON.parse(jsonStr);
        
        if (parsed.name === 'OpenClaw' && parsed.features.length === 3) {
            print('  ✅ JSON 处理成功');
            testResults.passed++;
            testResults.tests.push({ name: 'JSON处理', status: 'PASS' });
        } else {
            throw new Error('JSON 数据不匹配');
        }
        
        // 数组操作
        print('\n[3.2] 数组数据处理');
        const numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        const sum = numbers.reduce((a, b) => a + b, 0);
        const avg = sum / numbers.length;
        const filtered = numbers.filter(n => n > 5);
        
        print(`  数组求和: ${sum}`);
        print(`  平均值: ${avg}`);
        print(`  过滤结果: ${filtered.length} 个元素`);
        
        if (sum === 55 && avg === 5.5 && filtered.length === 5) {
            print('  ✅ 数组处理成功');
            testResults.passed++;
            testResults.tests.push({ name: '数组处理', status: 'PASS' });
        } else {
            throw new Error('数组计算结果错误');
        }
        
        // 字符串操作
        print('\n[3.3] 字符串数据处理');
        const text = 'OpenClaw WasmEdge Sandbox Test';
        const upper = text.toUpperCase();
        const words = text.split(' ');
        const reversed = text.split('').reverse().join('');
        
        print(`  大写转换: ${upper.substring(0, 20)}...`);
        print(`  单词分割: ${words.length} 个单词`);
        print(`  字符反转: ${reversed.substring(0, 20)}...`);
        
        if (words.length === 5 && upper.includes('OPENCLAW')) {
            print('  ✅ 字符串处理成功');
            testResults.passed++;
            testResults.tests.push({ name: '字符串处理', status: 'PASS' });
        } else {
            throw new Error('字符串处理结果错误');
        }
        
        return true;
    } catch (error) {
        print(`  ❌ 测试失败: ${error.message || error}`);
        testResults.failed++;
        testResults.tests.push({ name: '数据处理', status: 'FAIL', error: error.message });
        return false;
    }
}

// ============================================================
// 运行所有测试
// ============================================================
async function runAllTests() {
    print('\n开始运行测试案例...\n');
    
    await testFileOperations();
    await testNetworkRequest();
    testDataProcessing();
    
    // 生成测试报告
    print('\n\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    print('  测试结果汇总');
    print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
    
    for (const test of testResults.tests) {
        if (test.status === 'PASS') {
            print(`  ✅ ${test.name} - 通过`);
        } else {
            print(`  ❌ ${test.name} - 失败: ${test.error || '未知错误'}`);
        }
    }
    
    const total = testResults.passed + testResults.failed;
    const percentage = total > 0 ? Math.round((testResults.passed / total) * 100) : 0;
    
    print(`\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
    print(`  总计: ${testResults.passed}/${total} 测试通过 (${percentage}%)`);
    print(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`);
    
    // 保存测试报告
    try {
        const std = await import('std');
        const reportFile = std.open('/workspace/test_report.txt', 'w');
        if (reportFile) {
            reportFile.puts('OpenClaw WasmEdge 测试报告\n');
            reportFile.puts('='.repeat(60) + '\n\n');
            reportFile.puts(`测试时间: ${new Date().toISOString()}\n`);
            reportFile.puts(`通过: ${testResults.passed}\n`);
            reportFile.puts(`失败: ${testResults.failed}\n`);
            reportFile.puts(`成功率: ${percentage}%\n\n`);
            reportFile.puts('详细结果:\n');
            for (const test of testResults.tests) {
                reportFile.puts(`  ${test.status === 'PASS' ? '✅' : '❌'} ${test.name}\n`);
            }
            reportFile.close();
            print('[INFO] 测试报告已保存到: /workspace/test_report.txt\n');
        }
    } catch (error) {
        print(`[WARN] 无法保存测试报告: ${error.message || error}\n`);
    }
}

// 执行测试
runAllTests().catch(error => {
    print(`\n[ERROR] 测试执行失败: ${error.message || error}\n`);
});
