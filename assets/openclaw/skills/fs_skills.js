/**
 * OpenClaw+ File System Skills
 * 
 * Provides secure file system operations within the WasmEdge sandbox.
 * All operations are subject to WASI security policies and capability checks.
 * 
 * @module FsSkills
 * @version 1.0.0
 * @license MIT
 */

import * as std from 'std';
import * as os from 'os';
import { SkillClient } from '../sdk/skill_client.js';

/**
 * File system skill configuration
 */
const FS_CONFIG = {
    maxFileSize: 10 * 1024 * 1024, // 10MB
    allowedExtensions: ['.txt', '.json', '.log', '.md', '.csv'],
    blockedPaths: ['/etc', '/sys', '/proc'],
    encoding: 'utf-8'
};

/**
 * Check if a path is allowed
 * @param {string} path - Path to check
 * @returns {boolean} Whether the path is allowed
 */
function isPathAllowed(path) {
    // Check blocked paths
    for (const blocked of FS_CONFIG.blockedPaths) {
        if (path.startsWith(blocked)) {
            return false;
        }
    }
    
    // Check file extension
    const ext = path.substring(path.lastIndexOf('.'));
    if (ext && !FS_CONFIG.allowedExtensions.includes(ext)) {
        return false;
    }
    
    return true;
}

/**
 * Read a file from the file system
 * @param {string} path - Path to the file
 * @returns {Object} File content and metadata
 */
export function readFile(path) {
    print(`[FsSkills] Reading file: ${path}`);
    
    // Security check
    if (!isPathAllowed(path)) {
        throw new Error(`Access denied: ${path} is not allowed`);
    }
    
    try {
        const file = std.open(path, 'r');
        if (!file) {
            throw new Error(`Failed to open file: ${path}`);
        }
        
        const content = file.readAsString();
        file.close();
        
        return {
            success: true,
            path: path,
            content: content,
            size: content.length,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            path: path,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Write content to a file
 * @param {string} path - Path to the file
 * @param {string} content - Content to write
 * @param {Object} options - Write options
 * @returns {Object} Write result
 */
export function writeFile(path, content, options = {}) {
    print(`[FsSkills] Writing file: ${path} (${content.length} bytes)`);
    
    // Security check
    if (!isPathAllowed(path)) {
        throw new Error(`Access denied: ${path} is not allowed`);
    }
    
    // Size check
    if (content.length > FS_CONFIG.maxFileSize) {
        throw new Error(`File too large: ${content.length} bytes (max: ${FS_CONFIG.maxFileSize})`);
    }
    
    try {
        const mode = options.append ? 'a' : 'w';
        const file = std.open(path, mode);
        if (!file) {
            throw new Error(`Failed to open file for writing: ${path}`);
        }
        
        file.puts(content);
        file.close();
        
        return {
            success: true,
            path: path,
            bytesWritten: content.length,
            mode: mode,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            path: path,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * List files in a directory
 * @param {string} dirPath - Directory path
 * @returns {Object} List of files
 */
export function listDirectory(dirPath) {
    print(`[FsSkills] Listing directory: ${dirPath}`);
    
    // Security check
    if (!isPathAllowed(dirPath)) {
        throw new Error(`Access denied: ${dirPath} is not allowed`);
    }
    
    try {
        const dir = std.opendir(dirPath);
        if (!dir) {
            throw new Error(`Failed to open directory: ${dirPath}`);
        }
        
        const files = [];
        let entry;
        while ((entry = dir.readdir()) !== null) {
            if (entry.name !== '.' && entry.name !== '..') {
                files.push({
                    name: entry.name,
                    type: entry.type === 4 ? 'directory' : 'file'
                });
            }
        }
        dir.close();
        
        return {
            success: true,
            path: dirPath,
            files: files,
            count: files.length,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            path: dirPath,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Check if a file exists
 * @param {string} path - Path to check
 * @returns {Object} Existence check result
 */
export function fileExists(path) {
    print(`[FsSkills] Checking file existence: ${path}`);
    
    try {
        const file = std.open(path, 'r');
        if (file) {
            file.close();
            return {
                success: true,
                path: path,
                exists: true,
                timestamp: new Date().toISOString()
            };
        }
        return {
            success: true,
            path: path,
            exists: false,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: true,
            path: path,
            exists: false,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Delete a file
 * @param {string} path - Path to the file
 * @returns {Object} Deletion result
 */
export function deleteFile(path) {
    print(`[FsSkills] Deleting file: ${path}`);
    
    // Security check
    if (!isPathAllowed(path)) {
        throw new Error(`Access denied: ${path} is not allowed`);
    }
    
    try {
        os.remove(path);
        return {
            success: true,
            path: path,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            path: path,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Copy a file
 * @param {string} sourcePath - Source file path
 * @param {string} destPath - Destination file path
 * @returns {Object} Copy result
 */
export function copyFile(sourcePath, destPath) {
    print(`[FsSkills] Copying file: ${sourcePath} -> ${destPath}`);
    
    // Security checks
    if (!isPathAllowed(sourcePath) || !isPathAllowed(destPath)) {
        throw new Error('Access denied: one or both paths are not allowed');
    }
    
    try {
        // Read source
        const sourceFile = std.open(sourcePath, 'r');
        if (!sourceFile) {
            throw new Error(`Failed to open source file: ${sourcePath}`);
        }
        const content = sourceFile.readAsString();
        sourceFile.close();
        
        // Write destination
        const destFile = std.open(destPath, 'w');
        if (!destFile) {
            throw new Error(`Failed to open destination file: ${destPath}`);
        }
        destFile.puts(content);
        destFile.close();
        
        return {
            success: true,
            sourcePath: sourcePath,
            destPath: destPath,
            bytesCopied: content.length,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            sourcePath: sourcePath,
            destPath: destPath,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Get file statistics
 * @param {string} path - Path to the file
 * @returns {Object} File statistics
 */
export function getFileStats(path) {
    print(`[FsSkills] Getting file stats: ${path}`);
    
    try {
        const file = std.open(path, 'r');
        if (!file) {
            throw new Error(`File not found: ${path}`);
        }
        
        const content = file.readAsString();
        file.close();
        
        const lines = content.split('\n').length;
        const words = content.split(/\s+/).filter(w => w.length > 0).length;
        
        return {
            success: true,
            path: path,
            size: content.length,
            lines: lines,
            words: words,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            path: path,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

// Example usage
if (typeof print !== 'undefined') {
    print('[FsSkills] File system skills module loaded');
    print(`[FsSkills] Max file size: ${FS_CONFIG.maxFileSize} bytes`);
    print(`[FsSkills] Allowed extensions: ${FS_CONFIG.allowedExtensions.join(', ')}`);
}

// Export for CommonJS compatibility
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        readFile,
        writeFile,
        listDirectory,
        fileExists,
        deleteFile,
        copyFile,
        getFileStats
    };
}
