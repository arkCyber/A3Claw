/**
 * OpenClaw+ Web/Network Skills
 * 
 * Provides secure HTTP/HTTPS operations within the WasmEdge sandbox.
 * All operations are subject to security policies and network capability checks.
 * 
 * @module WebSkills
 * @version 1.0.0
 * @license MIT
 */

import * as std from 'std';
import * as os from 'os';
import { SkillClient } from '../sdk/skill_client.js';

/**
 * Web skill configuration
 */
const WEB_CONFIG = {
    timeout: 30000, // 30 seconds
    maxResponseSize: 5 * 1024 * 1024, // 5MB
    allowedDomains: ['api.github.com', 'httpbin.org', 'jsonplaceholder.typicode.com'],
    allowedProtocols: ['http:', 'https:'],
    userAgent: 'OpenClaw+/1.0.0 (WasmEdge)'
};

/**
 * Check if a URL is allowed
 * @param {string} url - URL to check
 * @returns {boolean} Whether the URL is allowed
 */
function isUrlAllowed(url) {
    try {
        const urlObj = new URL(url);
        
        // Check protocol
        if (!WEB_CONFIG.allowedProtocols.includes(urlObj.protocol)) {
            return false;
        }
        
        // Check domain whitelist
        const hostname = urlObj.hostname;
        const isAllowed = WEB_CONFIG.allowedDomains.some(domain => {
            return hostname === domain || hostname.endsWith('.' + domain);
        });
        
        return isAllowed;
    } catch (error) {
        return false;
    }
}

/**
 * Parse URL and extract components
 * @param {string} url - URL to parse
 * @returns {Object} Parsed URL components
 */
export function parseUrl(url) {
    print(`[WebSkills] Parsing URL: ${url}`);
    
    try {
        const urlObj = new URL(url);
        
        return {
            success: true,
            protocol: urlObj.protocol,
            hostname: urlObj.hostname,
            port: urlObj.port || (urlObj.protocol === 'https:' ? '443' : '80'),
            pathname: urlObj.pathname,
            search: urlObj.search,
            hash: urlObj.hash,
            href: urlObj.href,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            url: url,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Perform HTTP GET request
 * @param {string} url - URL to fetch
 * @param {Object} options - Request options
 * @returns {Object} Response data
 */
export function httpGet(url, options = {}) {
    print(`[WebSkills] HTTP GET: ${url}`);
    
    // Security check
    if (!isUrlAllowed(url)) {
        throw new Error(`Access denied: ${url} is not in the allowed domains list`);
    }
    
    // Note: This is a placeholder implementation
    // In a real WasmEdge environment with WASI-HTTP support, this would use:
    // - wasi_http module for HTTP requests
    // - or fetch API if available
    // - or external host function calls
    
    return {
        success: false,
        url: url,
        error: 'HTTP requests not yet implemented in this WasmEdge environment',
        note: 'Requires WASI-HTTP plugin or host function support',
        timestamp: new Date().toISOString()
    };
}

/**
 * Perform HTTP POST request
 * @param {string} url - URL to post to
 * @param {Object} data - Data to send
 * @param {Object} options - Request options
 * @returns {Object} Response data
 */
export function httpPost(url, data, options = {}) {
    print(`[WebSkills] HTTP POST: ${url}`);
    
    // Security check
    if (!isUrlAllowed(url)) {
        throw new Error(`Access denied: ${url} is not in the allowed domains list`);
    }
    
    // Validate data
    if (!data || typeof data !== 'object') {
        throw new Error('Invalid POST data: must be an object');
    }
    
    return {
        success: false,
        url: url,
        error: 'HTTP requests not yet implemented in this WasmEdge environment',
        note: 'Requires WASI-HTTP plugin or host function support',
        timestamp: new Date().toISOString()
    };
}

/**
 * Download a file from URL
 * @param {string} url - URL to download from
 * @param {string} savePath - Path to save the file
 * @returns {Object} Download result
 */
export function downloadFile(url, savePath) {
    print(`[WebSkills] Downloading: ${url} -> ${savePath}`);
    
    // Security check
    if (!isUrlAllowed(url)) {
        throw new Error(`Access denied: ${url} is not in the allowed domains list`);
    }
    
    return {
        success: false,
        url: url,
        savePath: savePath,
        error: 'File download not yet implemented in this WasmEdge environment',
        note: 'Requires WASI-HTTP plugin or host function support',
        timestamp: new Date().toISOString()
    };
}

/**
 * Make a JSON API request
 * @param {string} url - API endpoint URL
 * @param {Object} options - Request options
 * @returns {Object} API response
 */
export function jsonApiRequest(url, options = {}) {
    print(`[WebSkills] JSON API request: ${url}`);
    
    // Security check
    if (!isUrlAllowed(url)) {
        throw new Error(`Access denied: ${url} is not in the allowed domains list`);
    }
    
    const method = options.method || 'GET';
    const headers = options.headers || {};
    headers['Content-Type'] = 'application/json';
    headers['Accept'] = 'application/json';
    
    return {
        success: false,
        url: url,
        method: method,
        error: 'JSON API requests not yet implemented in this WasmEdge environment',
        note: 'Requires WASI-HTTP plugin or host function support',
        timestamp: new Date().toISOString()
    };
}

/**
 * Check if a URL is reachable
 * @param {string} url - URL to check
 * @returns {Object} Reachability result
 */
export function checkUrlReachable(url) {
    print(`[WebSkills] Checking URL reachability: ${url}`);
    
    // Security check
    if (!isUrlAllowed(url)) {
        return {
            success: true,
            url: url,
            reachable: false,
            reason: 'URL not in allowed domains list',
            timestamp: new Date().toISOString()
        };
    }
    
    return {
        success: false,
        url: url,
        error: 'URL reachability check not yet implemented',
        note: 'Requires network connectivity support',
        timestamp: new Date().toISOString()
    };
}

/**
 * Encode data for URL query string
 * @param {Object} params - Parameters to encode
 * @returns {string} Encoded query string
 */
export function encodeQueryString(params) {
    print(`[WebSkills] Encoding query string`);
    
    try {
        const pairs = [];
        for (const key in params) {
            if (params.hasOwnProperty(key)) {
                const value = params[key];
                pairs.push(encodeURIComponent(key) + '=' + encodeURIComponent(value));
            }
        }
        
        return {
            success: true,
            queryString: pairs.join('&'),
            paramCount: pairs.length,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Decode URL query string
 * @param {string} queryString - Query string to decode
 * @returns {Object} Decoded parameters
 */
export function decodeQueryString(queryString) {
    print(`[WebSkills] Decoding query string`);
    
    try {
        const params = {};
        const pairs = queryString.replace(/^\?/, '').split('&');
        
        for (const pair of pairs) {
            if (pair) {
                const [key, value] = pair.split('=');
                params[decodeURIComponent(key)] = decodeURIComponent(value || '');
            }
        }
        
        return {
            success: true,
            params: params,
            paramCount: Object.keys(params).length,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: false,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Validate URL format
 * @param {string} url - URL to validate
 * @returns {Object} Validation result
 */
export function validateUrl(url) {
    print(`[WebSkills] Validating URL: ${url}`);
    
    try {
        const urlObj = new URL(url);
        const isAllowed = isUrlAllowed(url);
        
        return {
            success: true,
            url: url,
            valid: true,
            allowed: isAllowed,
            protocol: urlObj.protocol,
            hostname: urlObj.hostname,
            timestamp: new Date().toISOString()
        };
    } catch (error) {
        return {
            success: true,
            url: url,
            valid: false,
            allowed: false,
            error: error.message,
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Get web skill configuration
 * @returns {Object} Current configuration
 */
export function getWebConfig() {
    return {
        success: true,
        config: {
            timeout: WEB_CONFIG.timeout,
            maxResponseSize: WEB_CONFIG.maxResponseSize,
            allowedDomains: WEB_CONFIG.allowedDomains,
            allowedProtocols: WEB_CONFIG.allowedProtocols,
            userAgent: WEB_CONFIG.userAgent
        },
        timestamp: new Date().toISOString()
    };
}

// Example usage
if (typeof print !== 'undefined') {
    print('[WebSkills] Web/Network skills module loaded');
    print(`[WebSkills] Timeout: ${WEB_CONFIG.timeout}ms`);
    print(`[WebSkills] Max response size: ${WEB_CONFIG.maxResponseSize} bytes`);
    print(`[WebSkills] Allowed domains: ${WEB_CONFIG.allowedDomains.length}`);
}

// Export for CommonJS compatibility
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        parseUrl,
        httpGet,
        httpPost,
        downloadFile,
        jsonApiRequest,
        checkUrlReachable,
        encodeQueryString,
        decodeQueryString,
        validateUrl,
        getWebConfig
    };
}
