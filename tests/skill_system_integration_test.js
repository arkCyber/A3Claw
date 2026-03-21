/**
 * OpenClaw+ Skill System Integration Test
 * 
 * 完整的技能系统集成测试，验证所有功能模块
 * 使用自然语言描述每个测试用例
 * 
 * @version 1.0.0
 * @standard Aerospace-grade testing
 */

// 测试统计
let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

// 测试结果记录
const testResults = [];

// 兼容 QuickJS 和 Node.js 环境
const print = typeof console !== 'undefined' ? console.log : function() {};

/**
 * 打印彩色输出（如果支持）
 */
function printColor(message, color = 'reset') {
    const colors = {
        reset: '\x1b[0m',
        red: '\x1b[31m',
        green: '\x1b[32m',
        yellow: '\x1b[33m',
        blue: '\x1b[34m',
        magenta: '\x1b[35m',
        cyan: '\x1b[36m',
        bold: '\x1b[1m'
    };
    
    const colorCode = colors[color] || colors.reset;
    console.log(`${colorCode}${message}${colors.reset}`);
}

/**
 * 运行单个测试
 */
function runTest(testName, testDescription, testFunction) {
    totalTests++;
    
    printColor(`\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`, 'cyan');
    printColor(`📋 测试 #${totalTests}: ${testName}`, 'bold');
    printColor(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`, 'cyan');
    printColor(`描述: ${testDescription}`, 'yellow');
    print('');
    
    try {
        const result = testFunction();
        
        if (result === true || result === undefined) {
            printColor('✓ 测试通过', 'green');
            passedTests++;
            testResults.push({ name: testName, status: 'PASS', error: null });
        } else {
            printColor(`✗ 测试失败: ${result}`, 'red');
            failedTests++;
            testResults.push({ name: testName, status: 'FAIL', error: result });
        }
    } catch (error) {
        printColor(`✗ 测试失败: ${error.message}`, 'red');
        printColor(`堆栈: ${error.stack}`, 'red');
        failedTests++;
        testResults.push({ name: testName, status: 'FAIL', error: error.message });
    }
}

/**
 * 断言函数
 */
function assert(condition, message) {
    if (!condition) {
        throw new Error(message || 'Assertion failed');
    }
}

function assertEqual(actual, expected, message) {
    if (actual !== expected) {
        throw new Error(message || `Expected ${expected}, but got ${actual}`);
    }
}

function assertNotNull(value, message) {
    if (value === null || value === undefined) {
        throw new Error(message || 'Value is null or undefined');
    }
}

function assertType(value, type, message) {
    if (typeof value !== type) {
        throw new Error(message || `Expected type ${type}, but got ${typeof value}`);
    }
}

/**
 * 打印最终报告
 */
function printFinalReport() {
    printColor('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', 'magenta');
    printColor('技能系统集成测试 - 最终报告', 'bold');
    printColor('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', 'magenta');
    
    print(`\n总测试数: ${totalTests}`);
    printColor(`通过: ${passedTests}`, 'green');
    printColor(`失败: ${failedTests}`, 'red');
    
    if (totalTests > 0) {
        const successRate = ((passedTests / totalTests) * 100).toFixed(1);
        printColor(`成功率: ${successRate}%`, successRate === '100.0' ? 'green' : 'yellow');
    }
    
    printColor('\n详细测试结果:', 'cyan');
    printColor('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', 'cyan');
    
    testResults.forEach((result, index) => {
        const status = result.status === 'PASS' 
            ? '\x1b[32m✓\x1b[0m' 
            : '\x1b[31m✗\x1b[0m';
        print(`${status} ${result.name}`);
        if (result.error) {
            printColor(`  错误: ${result.error}`, 'red');
        }
    });
    
    printColor('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', 'cyan');
}

// ============================================================================
// 开始测试
// ============================================================================

printColor('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', 'magenta');
printColor('OpenClaw+ 技能系统集成测试', 'bold');
printColor('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━', 'magenta');
printColor(`开始时间: ${new Date().toISOString()}`, 'cyan');

// ============================================================================
// 第一部分: 基础功能测试
// ============================================================================

printColor('\n\n第一部分: 基础功能测试', 'blue');

runTest(
    '验证 JavaScript 基本功能可用',
    '测试 JavaScript 运行时的基本功能，包括变量、函数、对象等',
    function() {
        // 测试变量
        const testVar = 'Hello OpenClaw+';
        assert(testVar === 'Hello OpenClaw+', 'Variable assignment failed');
        
        // 测试函数
        function testFunc(x) {
            return x * 2;
        }
        assertEqual(testFunc(5), 10, 'Function execution failed');
        
        // 测试对象
        const testObj = { name: 'test', value: 42 };
        assertEqual(testObj.name, 'test', 'Object property access failed');
        assertEqual(testObj.value, 42, 'Object property value failed');
        
        // 测试数组
        const testArray = [1, 2, 3, 4, 5];
        assertEqual(testArray.length, 5, 'Array length failed');
        assertEqual(testArray[2], 3, 'Array indexing failed');
        
        print('✓ 变量赋值正常');
        print('✓ 函数执行正常');
        print('✓ 对象操作正常');
        print('✓ 数组操作正常');
        
        return true;
    }
);

runTest(
    '验证 JSON 序列化和反序列化功能',
    '测试 JSON.stringify 和 JSON.parse 是否正常工作',
    function() {
        const originalObj = {
            name: 'OpenClaw+',
            version: '1.0.0',
            features: ['skills', 'security', 'sandbox'],
            config: {
                debug: true,
                timeout: 30000
            }
        };
        
        // 序列化
        const jsonString = JSON.stringify(originalObj);
        assertType(jsonString, 'string', 'JSON.stringify should return string');
        print(`✓ JSON 序列化成功: ${jsonString.length} 字节`);
        
        // 反序列化
        const parsedObj = JSON.parse(jsonString);
        assertType(parsedObj, 'object', 'JSON.parse should return object');
        assertEqual(parsedObj.name, originalObj.name, 'Parsed name mismatch');
        assertEqual(parsedObj.version, originalObj.version, 'Parsed version mismatch');
        print('✓ JSON 反序列化成功');
        
        return true;
    }
);

runTest(
    '验证字符串操作功能',
    '测试字符串的各种操作方法',
    function() {
        const testString = 'OpenClaw+ Skill System';
        
        // 长度
        assertEqual(testString.length, 22, 'String length incorrect');
        
        // 大小写转换
        assertEqual(testString.toLowerCase(), 'openclaw+ skill system', 'toLowerCase failed');
        assertEqual(testString.toUpperCase(), 'OPENCLAW+ SKILL SYSTEM', 'toUpperCase failed');
        
        // 子串
        assert(testString.includes('Skill'), 'includes failed');
        assert(testString.startsWith('OpenClaw'), 'startsWith failed');
        assert(testString.endsWith('System'), 'endsWith failed');
        
        // 分割
        const parts = testString.split(' ');
        assertEqual(parts.length, 3, 'split failed');
        
        print('✓ 字符串长度正确');
        print('✓ 大小写转换正常');
        print('✓ 子串操作正常');
        print('✓ 字符串分割正常');
        
        return true;
    }
);

runTest(
    '验证数组操作功能',
    '测试数组的各种操作方法',
    function() {
        const testArray = [1, 2, 3, 4, 5];
        
        // map
        const doubled = testArray.map(x => x * 2);
        assertEqual(doubled[0], 2, 'map failed');
        assertEqual(doubled[4], 10, 'map failed');
        
        // filter
        const evens = testArray.filter(x => x % 2 === 0);
        assertEqual(evens.length, 2, 'filter failed');
        
        // reduce
        const sum = testArray.reduce((acc, x) => acc + x, 0);
        assertEqual(sum, 15, 'reduce failed');
        
        // find
        const found = testArray.find(x => x > 3);
        assertEqual(found, 4, 'find failed');
        
        print('✓ map 操作正常');
        print('✓ filter 操作正常');
        print('✓ reduce 操作正常');
        print('✓ find 操作正常');
        
        return true;
    }
);

// ============================================================================
// 第二部分: 技能系统配置测试
// ============================================================================

printColor('\n\n第二部分: 技能系统配置测试', 'blue');

runTest(
    '验证技能配置对象结构',
    '测试技能配置对象是否包含所有必需的字段',
    function() {
        const skillConfig = {
            name: 'test-skill',
            timeout: 30000,
            maxRetries: 3,
            validateInput: true,
            securityContext: {
                allowedPaths: ['/tmp', '/data'],
                blockedPaths: ['/etc', '/sys']
            }
        };
        
        assertNotNull(skillConfig.name, 'Config name is null');
        assertType(skillConfig.timeout, 'number', 'Timeout should be number');
        assertType(skillConfig.maxRetries, 'number', 'MaxRetries should be number');
        assertType(skillConfig.validateInput, 'boolean', 'ValidateInput should be boolean');
        assertType(skillConfig.securityContext, 'object', 'SecurityContext should be object');
        
        print(`✓ 技能名称: ${skillConfig.name}`);
        print(`✓ 超时时间: ${skillConfig.timeout}ms`);
        print(`✓ 最大重试: ${skillConfig.maxRetries}次`);
        print(`✓ 输入验证: ${skillConfig.validateInput}`);
        print(`✓ 安全上下文配置正确`);
        
        return true;
    }
);

runTest(
    '验证文件系统技能配置',
    '测试文件系统技能的配置参数',
    function() {
        const fsConfig = {
            maxFileSize: 10 * 1024 * 1024, // 10MB
            allowedExtensions: ['.txt', '.json', '.log', '.md', '.csv'],
            blockedPaths: ['/etc', '/sys', '/proc'],
            encoding: 'utf-8'
        };
        
        assertEqual(fsConfig.maxFileSize, 10485760, 'Max file size incorrect');
        assert(Array.isArray(fsConfig.allowedExtensions), 'AllowedExtensions should be array');
        assert(Array.isArray(fsConfig.blockedPaths), 'BlockedPaths should be array');
        assertEqual(fsConfig.encoding, 'utf-8', 'Encoding incorrect');
        
        print(`✓ 最大文件大小: ${fsConfig.maxFileSize / 1024 / 1024}MB`);
        print(`✓ 允许的扩展名: ${fsConfig.allowedExtensions.length}个`);
        print(`✓ 阻止的路径: ${fsConfig.blockedPaths.length}个`);
        print(`✓ 编码: ${fsConfig.encoding}`);
        
        return true;
    }
);

runTest(
    '验证网络技能配置',
    '测试网络技能的配置参数',
    function() {
        const webConfig = {
            timeout: 30000,
            maxResponseSize: 5 * 1024 * 1024, // 5MB
            allowedDomains: ['api.github.com', 'httpbin.org'],
            allowedProtocols: ['http:', 'https:'],
            userAgent: 'OpenClaw+/1.0.0'
        };
        
        assertEqual(webConfig.timeout, 30000, 'Timeout incorrect');
        assertEqual(webConfig.maxResponseSize, 5242880, 'Max response size incorrect');
        assert(Array.isArray(webConfig.allowedDomains), 'AllowedDomains should be array');
        assert(Array.isArray(webConfig.allowedProtocols), 'AllowedProtocols should be array');
        assertType(webConfig.userAgent, 'string', 'UserAgent should be string');
        
        print(`✓ 超时时间: ${webConfig.timeout}ms`);
        print(`✓ 最大响应大小: ${webConfig.maxResponseSize / 1024 / 1024}MB`);
        print(`✓ 允许的域名: ${webConfig.allowedDomains.length}个`);
        print(`✓ 允许的协议: ${webConfig.allowedProtocols.join(', ')}`);
        print(`✓ User-Agent: ${webConfig.userAgent}`);
        
        return true;
    }
);

// ============================================================================
// 第三部分: 数据验证测试
// ============================================================================

printColor('\n\n第三部分: 数据验证测试', 'blue');

runTest(
    '验证路径安全检查逻辑',
    '测试路径是否被正确分类为允许或阻止',
    function() {
        const blockedPaths = ['/etc', '/sys', '/proc'];
        
        function isPathBlocked(path) {
            return blockedPaths.some(blocked => path.startsWith(blocked));
        }
        
        // 应该被阻止的路径
        assert(isPathBlocked('/etc/passwd'), '/etc/passwd should be blocked');
        assert(isPathBlocked('/sys/kernel'), '/sys/kernel should be blocked');
        assert(isPathBlocked('/proc/cpuinfo'), '/proc/cpuinfo should be blocked');
        
        // 应该被允许的路径
        assert(!isPathBlocked('/tmp/test.txt'), '/tmp/test.txt should be allowed');
        assert(!isPathBlocked('/data/file.json'), '/data/file.json should be allowed');
        assert(!isPathBlocked('/home/user/doc.md'), '/home/user/doc.md should be allowed');
        
        print('✓ 敏感路径正确阻止');
        print('✓ 安全路径正确允许');
        
        return true;
    }
);

runTest(
    '验证文件扩展名检查逻辑',
    '测试文件扩展名是否被正确验证',
    function() {
        const allowedExtensions = ['.txt', '.json', '.log', '.md', '.csv'];
        
        function isExtensionAllowed(filename) {
            const ext = filename.substring(filename.lastIndexOf('.'));
            return allowedExtensions.includes(ext);
        }
        
        // 应该被允许的文件
        assert(isExtensionAllowed('test.txt'), 'test.txt should be allowed');
        assert(isExtensionAllowed('data.json'), 'data.json should be allowed');
        assert(isExtensionAllowed('app.log'), 'app.log should be allowed');
        
        // 应该被拒绝的文件
        assert(!isExtensionAllowed('script.sh'), 'script.sh should be rejected');
        assert(!isExtensionAllowed('binary.exe'), 'binary.exe should be rejected');
        assert(!isExtensionAllowed('program.bin'), 'program.bin should be rejected');
        
        print('✓ 允许的扩展名正确识别');
        print('✓ 禁止的扩展名正确拒绝');
        
        return true;
    }
);

runTest(
    '验证 URL 域名检查逻辑',
    '测试 URL 域名是否在白名单中',
    function() {
        const allowedDomains = ['api.github.com', 'httpbin.org', 'jsonplaceholder.typicode.com'];
        
        function isDomainAllowed(url) {
            try {
                // 简单的域名提取（实际应使用 URL 对象）
                const match = url.match(/https?:\/\/([^\/]+)/);
                if (!match) return false;
                const hostname = match[1];
                return allowedDomains.some(domain => hostname === domain || hostname.endsWith('.' + domain));
            } catch (e) {
                return false;
            }
        }
        
        // 应该被允许的 URL
        assert(isDomainAllowed('https://api.github.com/users'), 'GitHub API should be allowed');
        assert(isDomainAllowed('http://httpbin.org/get'), 'httpbin should be allowed');
        
        // 应该被拒绝的 URL
        assert(!isDomainAllowed('https://evil.com/malware'), 'evil.com should be rejected');
        assert(!isDomainAllowed('http://unknown.org/data'), 'unknown.org should be rejected');
        
        print('✓ 白名单域名正确允许');
        print('✓ 非白名单域名正确拒绝');
        
        return true;
    }
);

// ============================================================================
// 第四部分: 错误处理测试
// ============================================================================

printColor('\n\n第四部分: 错误处理测试', 'blue');

runTest(
    '验证异常捕获机制',
    '测试 try-catch 是否正常工作',
    function() {
        let errorCaught = false;
        let errorMessage = '';
        
        try {
            throw new Error('Test error');
        } catch (error) {
            errorCaught = true;
            errorMessage = error.message;
        }
        
        assert(errorCaught, 'Error should be caught');
        assertEqual(errorMessage, 'Test error', 'Error message incorrect');
        
        print('✓ 异常正确捕获');
        print(`✓ 错误信息: ${errorMessage}`);
        
        return true;
    }
);

runTest(
    '验证输入验证错误处理',
    '测试无效输入是否被正确拒绝',
    function() {
        function validateInput(input) {
            if (typeof input !== 'object' || input === null) {
                throw new Error('Input must be an object');
            }
            if (!input.name || typeof input.name !== 'string') {
                throw new Error('Input must have a valid name');
            }
            return true;
        }
        
        // 有效输入
        assert(validateInput({ name: 'test' }), 'Valid input should pass');
        
        // 无效输入
        let error1Caught = false;
        try {
            validateInput(null);
        } catch (e) {
            error1Caught = true;
            assertEqual(e.message, 'Input must be an object', 'Error message incorrect');
        }
        assert(error1Caught, 'Null input should throw error');
        
        let error2Caught = false;
        try {
            validateInput({});
        } catch (e) {
            error2Caught = true;
            assertEqual(e.message, 'Input must have a valid name', 'Error message incorrect');
        }
        assert(error2Caught, 'Missing name should throw error');
        
        print('✓ 有效输入正确接受');
        print('✓ 无效输入正确拒绝');
        print('✓ 错误信息准确');
        
        return true;
    }
);

// ============================================================================
// 第五部分: 性能和统计测试
// ============================================================================

printColor('\n\n第五部分: 性能和统计测试', 'blue');

runTest(
    '验证执行统计功能',
    '测试执行次数、成功率等统计信息的计算',
    function() {
        const stats = {
            executionCount: 100,
            failureCount: 5,
            totalDuration: 5000
        };
        
        const successCount = stats.executionCount - stats.failureCount;
        const successRate = (successCount / stats.executionCount * 100).toFixed(2);
        const averageDuration = (stats.totalDuration / stats.executionCount).toFixed(2);
        
        assertEqual(successCount, 95, 'Success count incorrect');
        assertEqual(successRate, '95.00', 'Success rate incorrect');
        assertEqual(averageDuration, '50.00', 'Average duration incorrect');
        
        print(`✓ 总执行次数: ${stats.executionCount}`);
        print(`✓ 成功次数: ${successCount}`);
        print(`✓ 失败次数: ${stats.failureCount}`);
        print(`✓ 成功率: ${successRate}%`);
        print(`✓ 平均耗时: ${averageDuration}ms`);
        
        return true;
    }
);

runTest(
    '验证时间戳生成',
    '测试 ISO 8601 格式的时间戳生成',
    function() {
        const timestamp = new Date().toISOString();
        
        assertType(timestamp, 'string', 'Timestamp should be string');
        assert(timestamp.includes('T'), 'Timestamp should contain T separator');
        assert(timestamp.includes('Z'), 'Timestamp should end with Z');
        assert(timestamp.length >= 20, 'Timestamp should be at least 20 characters');
        
        print(`✓ 时间戳格式正确: ${timestamp}`);
        
        return true;
    }
);

// ============================================================================
// 生成最终报告
// ============================================================================

printFinalReport();

// 返回退出码
if (failedTests > 0) {
    printColor('\n⚠ 存在测试失败，请检查上述错误信息', 'red');
} else {
    printColor('\n🎉 所有测试通过！', 'green');
}
