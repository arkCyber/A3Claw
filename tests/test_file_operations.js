#!/usr/bin/env node
/**
 * OpenClaw 文件操作功能测试
 * 测试文件读写、目录操作、文件搜索等功能
 */

// 导入标准库（WasmEdge QuickJS）
const std = await import('std');

// 测试配置
const TEST_DIR = '/workspace/test_files';
const TEST_FILES = [
  { name: 'test1.txt', content: 'Hello OpenClaw! This is a test file.' },
  { name: 'test2.json', content: JSON.stringify({ message: 'Test data', timestamp: Date.now() }) },
  { name: 'test3.md', content: '# Test Document\n\nThis is a markdown test file.' }
];

// 文件操作函数
function writeFile(path, content) {
  try {
    const file = std.open(path, 'w');
    if (!file) {
      throw new Error(`无法打开文件: ${path}`);
    }
    file.puts(content);
    file.close();
    return true;
  } catch (error) {
    print(`[ERROR] 写入文件失败: ${error.message || error}`);
    return false;
  }
}

function readFile(path) {
  try {
    const file = std.open(path, 'r');
    if (!file) {
      throw new Error(`无法打开文件: ${path}`);
    }
    
    let content = '';
    let line;
    while ((line = file.getline()) !== null) {
      content += line + '\n';
    }
    file.close();
    
    return content.trim();
  } catch (error) {
    print(`[ERROR] 读取文件失败: ${error.message || error}`);
    return null;
  }
}

function fileExists(path) {
  try {
    const file = std.open(path, 'r');
    if (file) {
      file.close();
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

// 运行文件操作测试
function runFileTests() {
  print('\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  print('  OpenClaw 文件操作功能测试');
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  const results = {
    write: [],
    read: [],
    verify: []
  };
  
  // 测试 1: 文件写入
  print('[TEST 1] 文件写入功能测试');
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  for (const testFile of TEST_FILES) {
    const filePath = `${TEST_DIR}/${testFile.name}`;
    print(`[WRITE] ${filePath}`);
    print(`  内容长度: ${testFile.content.length} 字节`);
    
    const success = writeFile(filePath, testFile.content);
    
    if (success) {
      print(`  [SUCCESS] ✅ 文件写入成功\n`);
      results.write.push({ file: testFile.name, status: 'PASS' });
    } else {
      print(`  [FAIL] ❌ 文件写入失败\n`);
      results.write.push({ file: testFile.name, status: 'FAIL' });
    }
  }
  
  // 测试 2: 文件读取
  print('\n[TEST 2] 文件读取功能测试');
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  for (const testFile of TEST_FILES) {
    const filePath = `${TEST_DIR}/${testFile.name}`;
    print(`[READ] ${filePath}`);
    
    const content = readFile(filePath);
    
    if (content !== null) {
      print(`  读取长度: ${content.length} 字节`);
      print(`  预览: ${content.substring(0, 50)}...`);
      print(`  [SUCCESS] ✅ 文件读取成功\n`);
      results.read.push({ file: testFile.name, status: 'PASS', size: content.length });
    } else {
      print(`  [FAIL] ❌ 文件读取失败\n`);
      results.read.push({ file: testFile.name, status: 'FAIL' });
    }
  }
  
  // 测试 3: 内容验证
  print('\n[TEST 3] 文件内容验证测试');
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  for (const testFile of TEST_FILES) {
    const filePath = `${TEST_DIR}/${testFile.name}`;
    print(`[VERIFY] ${filePath}`);
    
    const content = readFile(filePath);
    
    if (content !== null && content === testFile.content) {
      print(`  [SUCCESS] ✅ 内容验证通过\n`);
      results.verify.push({ file: testFile.name, status: 'PASS' });
    } else {
      print(`  [FAIL] ❌ 内容不匹配\n`);
      results.verify.push({ file: testFile.name, status: 'FAIL' });
    }
  }
  
  // 测试 4: 文件存在性检查
  print('\n[TEST 4] 文件存在性检查');
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  for (const testFile of TEST_FILES) {
    const filePath = `${TEST_DIR}/${testFile.name}`;
    const exists = fileExists(filePath);
    print(`[CHECK] ${filePath}: ${exists ? '✅ 存在' : '❌ 不存在'}`);
  }
  
  // 生成测试报告
  print('\n\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
  print('  测试结果汇总');
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  const writeSuccess = results.write.filter(r => r.status === 'PASS').length;
  const readSuccess = results.read.filter(r => r.status === 'PASS').length;
  const verifySuccess = results.verify.filter(r => r.status === 'PASS').length;
  
  print(`文件写入: ${writeSuccess}/${TEST_FILES.length} 通过`);
  print(`文件读取: ${readSuccess}/${TEST_FILES.length} 通过`);
  print(`内容验证: ${verifySuccess}/${TEST_FILES.length} 通过`);
  
  const totalTests = TEST_FILES.length * 3;
  const totalSuccess = writeSuccess + readSuccess + verifySuccess;
  
  print(`\n总计: ${totalSuccess}/${totalTests} 测试通过`);
  print('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n');
  
  return results;
}

// 执行测试
print('[INFO] 开始文件操作测试...\n');

try {
  const results = runFileTests();
  print('[INFO] 文件操作测试完成');
} catch (error) {
  print(`[ERROR] 测试执行失败: ${error.message || error}`);
}
