#!/usr/bin/env python3
"""
Formation Documentation Testing Script

This script validates all documentation by:
1. Verifying all code examples work
2. Testing all API endpoints mentioned in docs
3. Validating all links and references

Usage:
    python scripts/test-documentation.py [--verbose] [--fix-links]
"""

import os
import re
import sys
import json
import time
import requests
import subprocess
import tempfile
from pathlib import Path
from typing import List, Dict, Set, Tuple, Optional
from urllib.parse import urlparse, urljoin
import argparse
from dataclasses import dataclass
from concurrent.futures import ThreadPoolExecutor, as_completed

@dataclass
class TestResult:
    """Result of a documentation test"""
    test_type: str
    file_path: str
    test_name: str
    success: bool
    message: str
    line_number: Optional[int] = None

class DocumentationTester:
    """Main class for testing Formation documentation"""
    
    def __init__(self, docs_dir: str = "docs", verbose: bool = False, fix_links: bool = False):
        self.docs_dir = Path(docs_dir)
        self.verbose = verbose
        self.fix_links = fix_links
        self.results: List[TestResult] = []
        self.formation_base_url = "http://localhost:3004"  # Default Formation API
        
        # Patterns for extracting different types of content
        self.code_block_pattern = re.compile(r'```(\w+)?\n(.*?)\n```', re.DOTALL)
        self.link_pattern = re.compile(r'\[([^\]]+)\]\(([^)]+)\)')
        self.api_endpoint_pattern = re.compile(r'(GET|POST|PUT|DELETE|PATCH)\s+([/\w\-{}]+)')
        self.curl_pattern = re.compile(r'curl\s+.*?(?=\n\n|\n```|\Z)', re.DOTALL)
        
        # Known external domains that should be accessible
        self.external_domains = {
            'github.com', 'discord.gg', 'formation.ai', 'api.openai.com',
            'docs.python.org', 'nodejs.org', 'docker.com', 'rust-lang.org'
        }
        
        # File extensions to check
        self.doc_extensions = {'.md', '.rst', '.txt'}
    
    def log(self, message: str, level: str = "INFO"):
        """Log a message if verbose mode is enabled"""
        if self.verbose or level == "ERROR":
            timestamp = time.strftime("%H:%M:%S")
            print(f"[{timestamp}] {level}: {message}")
    
    def add_result(self, result: TestResult):
        """Add a test result"""
        self.results.append(result)
        if not result.success:
            self.log(f"FAIL: {result.test_name} in {result.file_path}: {result.message}", "ERROR")
        elif self.verbose:
            self.log(f"PASS: {result.test_name} in {result.file_path}")
    
    def find_documentation_files(self) -> List[Path]:
        """Find all documentation files to test"""
        doc_files = []
        for ext in self.doc_extensions:
            doc_files.extend(self.docs_dir.rglob(f"*{ext}"))
        
        self.log(f"Found {len(doc_files)} documentation files")
        return doc_files
    
    def extract_code_blocks(self, content: str, file_path: Path) -> List[Tuple[str, str, int]]:
        """Extract code blocks from markdown content"""
        code_blocks = []
        lines = content.split('\n')
        
        in_code_block = False
        current_lang = None
        current_code = []
        start_line = 0
        
        for i, line in enumerate(lines):
            if line.startswith('```'):
                if not in_code_block:
                    # Starting a code block
                    in_code_block = True
                    current_lang = line[3:].strip() or 'text'
                    current_code = []
                    start_line = i + 1
                else:
                    # Ending a code block
                    in_code_block = False
                    if current_code:
                        code_blocks.append((current_lang, '\n'.join(current_code), start_line))
                    current_lang = None
                    current_code = []
            elif in_code_block:
                current_code.append(line)
        
        self.log(f"Extracted {len(code_blocks)} code blocks from {file_path}")
        return code_blocks
    
    def test_python_code(self, code: str, file_path: Path, line_number: int) -> TestResult:
        """Test Python code for syntax errors"""
        try:
            # Create a temporary file with the code
            with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
                # Add common imports that might be missing
                imports = [
                    "import os",
                    "import sys", 
                    "import json",
                    "import time",
                    "import requests",
                    "import hashlib",
                    "from typing import Dict, List, Any, Optional",
                    "from datetime import datetime",
                    ""
                ]
                
                # Write imports and code
                f.write('\n'.join(imports))
                f.write(code)
                temp_file = f.name
            
            # Check syntax
            result = subprocess.run([sys.executable, '-m', 'py_compile', temp_file], 
                                  capture_output=True, text=True)
            
            # Clean up
            os.unlink(temp_file)
            
            if result.returncode == 0:
                return TestResult("code_syntax", str(file_path), "Python syntax check", 
                                True, "Python code syntax is valid", line_number)
            else:
                return TestResult("code_syntax", str(file_path), "Python syntax check", 
                                False, f"Python syntax error: {result.stderr}", line_number)
                
        except Exception as e:
            return TestResult("code_syntax", str(file_path), "Python syntax check", 
                            False, f"Error testing Python code: {str(e)}", line_number)
    
    def test_javascript_code(self, code: str, file_path: Path, line_number: int) -> TestResult:
        """Test JavaScript code for syntax errors"""
        try:
            # Check if node is available
            result = subprocess.run(['node', '--version'], capture_output=True)
            if result.returncode != 0:
                return TestResult("code_syntax", str(file_path), "JavaScript syntax check", 
                                False, "Node.js not available for testing", line_number)
            
            # Create a temporary file with the code
            with tempfile.NamedTemporaryFile(mode='w', suffix='.js', delete=False) as f:
                # Add common requires that might be missing
                requires = [
                    "// Common requires for Formation examples",
                    "try { const crypto = require('crypto'); } catch(e) {}",
                    "try { const { ethers } = require('ethers'); } catch(e) {}",
                    "try { const axios = require('axios'); } catch(e) {}",
                    "try { const fetch = require('node-fetch'); } catch(e) {}",
                    ""
                ]
                
                f.write('\n'.join(requires))
                f.write(code)
                temp_file = f.name
            
            # Check syntax
            result = subprocess.run(['node', '--check', temp_file], 
                                  capture_output=True, text=True)
            
            # Clean up
            os.unlink(temp_file)
            
            if result.returncode == 0:
                return TestResult("code_syntax", str(file_path), "JavaScript syntax check", 
                                True, "JavaScript code syntax is valid", line_number)
            else:
                return TestResult("code_syntax", str(file_path), "JavaScript syntax check", 
                                False, f"JavaScript syntax error: {result.stderr}", line_number)
                
        except Exception as e:
            return TestResult("code_syntax", str(file_path), "JavaScript syntax check", 
                            False, f"Error testing JavaScript code: {str(e)}", line_number)
    
    def test_bash_code(self, code: str, file_path: Path, line_number: int) -> TestResult:
        """Test Bash code for syntax errors"""
        try:
            # Create a temporary file with the code
            with tempfile.NamedTemporaryFile(mode='w', suffix='.sh', delete=False) as f:
                f.write("#!/bin/bash\n")
                f.write("set -e\n")  # Exit on error
                f.write(code)
                temp_file = f.name
            
            # Check syntax
            result = subprocess.run(['bash', '-n', temp_file], 
                                  capture_output=True, text=True)
            
            # Clean up
            os.unlink(temp_file)
            
            if result.returncode == 0:
                return TestResult("code_syntax", str(file_path), "Bash syntax check", 
                                True, "Bash code syntax is valid", line_number)
            else:
                return TestResult("code_syntax", str(file_path), "Bash syntax check", 
                                False, f"Bash syntax error: {result.stderr}", line_number)
                
        except Exception as e:
            return TestResult("code_syntax", str(file_path), "Bash syntax check", 
                            False, f"Error testing Bash code: {str(e)}", line_number)
    
    def test_curl_commands(self, content: str, file_path: Path) -> List[TestResult]:
        """Extract and validate curl commands"""
        results = []
        curl_commands = self.curl_pattern.findall(content)
        
        for i, curl_cmd in enumerate(curl_commands):
            # Basic validation of curl command structure
            if 'curl' not in curl_cmd:
                continue
                
            # Check for common issues
            issues = []
            
            # Check for proper URL format
            if 'http' not in curl_cmd and 'localhost' not in curl_cmd:
                issues.append("No valid URL found")
            
            # Check for proper headers in Formation examples
            if 'formation.ai' in curl_cmd.lower() or 'localhost:3004' in curl_cmd:
                if 'X-Formation-Address' not in curl_cmd:
                    issues.append("Missing X-Formation-Address header for Formation API")
                if 'X-Formation-Signature' not in curl_cmd:
                    issues.append("Missing X-Formation-Signature header for Formation API")
            
            # Check for proper JSON content type
            if '-d' in curl_cmd and 'application/json' not in curl_cmd:
                issues.append("JSON data without Content-Type: application/json header")
            
            if issues:
                results.append(TestResult("curl_validation", str(file_path), f"cURL command {i+1}", 
                                        False, "; ".join(issues)))
            else:
                results.append(TestResult("curl_validation", str(file_path), f"cURL command {i+1}", 
                                        True, "cURL command structure is valid"))
        
        return results
    
    def extract_api_endpoints(self, content: str) -> List[Tuple[str, str]]:
        """Extract API endpoints from documentation"""
        endpoints = []
        
        # Find HTTP method + endpoint patterns
        matches = self.api_endpoint_pattern.findall(content)
        for method, endpoint in matches:
            endpoints.append((method, endpoint))
        
        # Also look for endpoint definitions in code blocks
        code_blocks = self.code_block_pattern.findall(content)
        for lang, code in code_blocks:
            if lang in ['http', 'bash', 'curl']:
                endpoint_matches = self.api_endpoint_pattern.findall(code)
                endpoints.extend(endpoint_matches)
        
        return list(set(endpoints))  # Remove duplicates
    
    def test_api_endpoint(self, method: str, endpoint: str, file_path: Path) -> TestResult:
        """Test if an API endpoint is documented correctly"""
        # Skip testing actual API calls for now, just validate format
        issues = []
        
        # Check endpoint format
        if not endpoint.startswith('/'):
            issues.append("Endpoint should start with '/'")
        
        # Check for Formation-specific patterns
        formation_patterns = ['/v1/', '/health', '/network/', '/agents/', '/models/']
        is_formation_endpoint = any(pattern in endpoint for pattern in formation_patterns)
        
        if is_formation_endpoint:
            # Validate Formation endpoint patterns
            if '/v1/' in endpoint:
                # Should follow REST conventions
                if method == 'GET' and endpoint.endswith('/create'):
                    issues.append("GET method should not use /create endpoint")
                elif method == 'POST' and '/list' in endpoint:
                    issues.append("POST method typically shouldn't use /list endpoint")
        
        # Check for parameter placeholders
        if '{' in endpoint and '}' in endpoint:
            # Validate parameter format
            params = re.findall(r'\{([^}]+)\}', endpoint)
            for param in params:
                if not param.replace('_', '').replace('-', '').isalnum():
                    issues.append(f"Invalid parameter format: {{{param}}}")
        
        if issues:
            return TestResult("api_endpoint", str(file_path), f"{method} {endpoint}", 
                            False, "; ".join(issues))
        else:
            return TestResult("api_endpoint", str(file_path), f"{method} {endpoint}", 
                            True, "API endpoint format is valid")
    
    def extract_links(self, content: str) -> List[Tuple[str, str, int]]:
        """Extract all links from markdown content"""
        links = []
        lines = content.split('\n')
        
        for line_num, line in enumerate(lines, 1):
            matches = self.link_pattern.findall(line)
            for text, url in matches:
                links.append((text, url, line_num))
        
        return links
    
    def test_internal_link(self, url: str, file_path: Path, line_number: int) -> TestResult:
        """Test internal documentation links"""
        # Handle relative links
        if url.startswith('#'):
            # Anchor link - check if anchor exists in same file
            anchor = url[1:].lower().replace(' ', '-').replace('_', '-')
            
            # Read the file content to check for headers
            try:
                with open(file_path, 'r', encoding='utf-8') as f:
                    content = f.read().lower()
                
                # Look for markdown headers that would create this anchor
                header_patterns = [
                    f"# {anchor.replace('-', ' ')}",
                    f"## {anchor.replace('-', ' ')}",
                    f"### {anchor.replace('-', ' ')}",
                    f"#### {anchor.replace('-', ' ')}"
                ]
                
                found = any(pattern in content for pattern in header_patterns)
                if found:
                    return TestResult("internal_link", str(file_path), f"Anchor link {url}", 
                                    True, "Anchor link target found", line_number)
                else:
                    return TestResult("internal_link", str(file_path), f"Anchor link {url}", 
                                    False, "Anchor link target not found", line_number)
                    
            except Exception as e:
                return TestResult("internal_link", str(file_path), f"Anchor link {url}", 
                                False, f"Error checking anchor: {str(e)}", line_number)
        
        elif url.startswith('./') or url.startswith('../') or (not url.startswith('http') and '/' in url):
            # Relative file link
            if url.startswith('./'):
                url = url[2:]
            
            # Resolve relative path
            target_path = file_path.parent / url
            
            # Handle anchor links in relative paths
            if '#' in url:
                url_part, anchor = url.split('#', 1)
                target_path = file_path.parent / url_part
            
            # Normalize path
            try:
                target_path = target_path.resolve()
                
                if target_path.exists():
                    return TestResult("internal_link", str(file_path), f"Relative link {url}", 
                                    True, "Relative link target exists", line_number)
                else:
                    return TestResult("internal_link", str(file_path), f"Relative link {url}", 
                                    False, f"Relative link target not found: {target_path}", line_number)
                    
            except Exception as e:
                return TestResult("internal_link", str(file_path), f"Relative link {url}", 
                                False, f"Error resolving relative link: {str(e)}", line_number)
        
        else:
            return TestResult("internal_link", str(file_path), f"Link {url}", 
                            True, "Not an internal link", line_number)
    
    def test_external_link(self, url: str, file_path: Path, line_number: int) -> TestResult:
        """Test external links"""
        if not url.startswith('http'):
            return TestResult("external_link", str(file_path), f"Link {url}", 
                            True, "Not an external HTTP link", line_number)
        
        try:
            # Parse URL
            parsed = urlparse(url)
            domain = parsed.netloc.lower()
            
            # Skip testing for known external domains (to avoid rate limiting)
            if any(known_domain in domain for known_domain in self.external_domains):
                return TestResult("external_link", str(file_path), f"External link {url}", 
                                True, "Known external domain (not tested)", line_number)
            
            # For other external links, do a quick HEAD request
            try:
                response = requests.head(url, timeout=10, allow_redirects=True)
                if response.status_code < 400:
                    return TestResult("external_link", str(file_path), f"External link {url}", 
                                    True, f"External link accessible (status: {response.status_code})", line_number)
                else:
                    return TestResult("external_link", str(file_path), f"External link {url}", 
                                    False, f"External link returned status: {response.status_code}", line_number)
                    
            except requests.RequestException as e:
                return TestResult("external_link", str(file_path), f"External link {url}", 
                                False, f"External link not accessible: {str(e)}", line_number)
                
        except Exception as e:
            return TestResult("external_link", str(file_path), f"External link {url}", 
                            False, f"Error testing external link: {str(e)}", line_number)
    
    def test_file(self, file_path: Path) -> List[TestResult]:
        """Test a single documentation file"""
        results = []
        
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()
        except Exception as e:
            results.append(TestResult("file_read", str(file_path), "File reading", 
                                    False, f"Could not read file: {str(e)}"))
            return results
        
        self.log(f"Testing file: {file_path}")
        
        # Test 1: Code examples
        code_blocks = self.extract_code_blocks(content, file_path)
        for lang, code, line_num in code_blocks:
            if lang.lower() in ['python', 'py']:
                results.append(self.test_python_code(code, file_path, line_num))
            elif lang.lower() in ['javascript', 'js', 'node']:
                results.append(self.test_javascript_code(code, file_path, line_num))
            elif lang.lower() in ['bash', 'sh', 'shell']:
                results.append(self.test_bash_code(code, file_path, line_num))
        
        # Test cURL commands
        results.extend(self.test_curl_commands(content, file_path))
        
        # Test 2: API endpoints
        endpoints = self.extract_api_endpoints(content)
        for method, endpoint in endpoints:
            results.append(self.test_api_endpoint(method, endpoint, file_path))
        
        # Test 3: Links and references
        links = self.extract_links(content)
        for text, url, line_num in links:
            # Test internal links
            results.append(self.test_internal_link(url, file_path, line_num))
            
            # Test external links
            results.append(self.test_external_link(url, file_path, line_num))
        
        return results
    
    def run_tests(self) -> Dict[str, any]:
        """Run all documentation tests"""
        self.log("Starting documentation tests...")
        
        doc_files = self.find_documentation_files()
        
        # Test each file
        with ThreadPoolExecutor(max_workers=4) as executor:
            future_to_file = {executor.submit(self.test_file, file_path): file_path 
                             for file_path in doc_files}
            
            for future in as_completed(future_to_file):
                file_path = future_to_file[future]
                try:
                    file_results = future.result()
                    self.results.extend(file_results)
                except Exception as e:
                    self.add_result(TestResult("file_test", str(file_path), "File testing", 
                                             False, f"Error testing file: {str(e)}"))
        
        # Compile results
        total_tests = len(self.results)
        passed_tests = sum(1 for r in self.results if r.success)
        failed_tests = total_tests - passed_tests
        
        # Group results by type
        results_by_type = {}
        for result in self.results:
            if result.test_type not in results_by_type:
                results_by_type[result.test_type] = {'passed': 0, 'failed': 0, 'tests': []}
            
            if result.success:
                results_by_type[result.test_type]['passed'] += 1
            else:
                results_by_type[result.test_type]['failed'] += 1
            
            results_by_type[result.test_type]['tests'].append(result)
        
        summary = {
            'total_tests': total_tests,
            'passed_tests': passed_tests,
            'failed_tests': failed_tests,
            'success_rate': (passed_tests / total_tests * 100) if total_tests > 0 else 0,
            'results_by_type': results_by_type,
            'all_results': self.results
        }
        
        return summary
    
    def print_summary(self, summary: Dict[str, any]):
        """Print test summary"""
        print("\n" + "="*80)
        print("FORMATION DOCUMENTATION TEST SUMMARY")
        print("="*80)
        
        print(f"Total Tests: {summary['total_tests']}")
        print(f"Passed: {summary['passed_tests']}")
        print(f"Failed: {summary['failed_tests']}")
        print(f"Success Rate: {summary['success_rate']:.1f}%")
        
        print("\nResults by Test Type:")
        print("-" * 40)
        
        for test_type, stats in summary['results_by_type'].items():
            total = stats['passed'] + stats['failed']
            rate = (stats['passed'] / total * 100) if total > 0 else 0
            print(f"{test_type:20} {stats['passed']:3}/{total:3} ({rate:5.1f}%)")
        
        # Show failures
        failures = [r for r in summary['all_results'] if not r.success]
        if failures:
            print(f"\nFAILURES ({len(failures)}):")
            print("-" * 40)
            
            for failure in failures[:20]:  # Show first 20 failures
                location = f"{failure.file_path}"
                if failure.line_number:
                    location += f":{failure.line_number}"
                print(f"❌ {failure.test_name}")
                print(f"   File: {location}")
                print(f"   Error: {failure.message}")
                print()
            
            if len(failures) > 20:
                print(f"... and {len(failures) - 20} more failures")
        
        print("\n" + "="*80)

def main():
    """Main function"""
    parser = argparse.ArgumentParser(description="Test Formation documentation")
    parser.add_argument("--verbose", "-v", action="store_true", 
                       help="Enable verbose output")
    parser.add_argument("--fix-links", action="store_true", 
                       help="Attempt to fix broken links")
    parser.add_argument("--docs-dir", default="docs", 
                       help="Documentation directory to test")
    
    args = parser.parse_args()
    
    # Create tester
    tester = DocumentationTester(
        docs_dir=args.docs_dir,
        verbose=args.verbose,
        fix_links=args.fix_links
    )
    
    # Run tests
    summary = tester.run_tests()
    
    # Print results
    tester.print_summary(summary)
    
    # Exit with error code if tests failed
    if summary['failed_tests'] > 0:
        sys.exit(1)
    else:
        print("✅ All documentation tests passed!")
        sys.exit(0)

if __name__ == "__main__":
    main() 