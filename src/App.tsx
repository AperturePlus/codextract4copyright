import { useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Box, Button, TextField, Typography, Paper, Stack, Checkbox, IconButton, CircularProgress, Autocomplete, Chip, LinearProgress } from '@mui/material';
import DeleteIcon from '@mui/icons-material/Delete';
import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import SearchIcon from '@mui/icons-material/Search';
import { FileTree, FileInfo } from './components/FileTree';

function buildDocumentTitle(softwareName: string, softwareVersion: string) {
  const name = softwareName.trim();
  if (!name) {
    return "源代码提取报告";
  }

  const version = softwareVersion.trim() || "V1.0";
  return `《${name}》${version}源代码`;
}

function sanitizeWindowsFileName(fileName: string) {
  const sanitized = fileName
    .replace(/[<>:"\/\\|?*\x00-\x1F]/g, "_")
    .replace(/[. ]+$/g, "")
    .trim();

  return sanitized || "源代码提取报告";
}

function App() {
  const [rootPath, setRootPath] = useState("");
  const [excludeRules, setExcludeRules] = useState<string[]>([".git", "target", "node_modules", "bin", "obj", ".vs", ".idea"]);
  const [newRule, setNewRule] = useState("");
  const [selectedExtensions, setSelectedExtensions] = useState<string[]>([]);

  const [files, setFiles] = useState<FileInfo[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());

  const [isScanning, setIsScanning] = useState(false);
  const [isExtracting, setIsExtracting] = useState(false);

  const [softwareName, setSoftwareName] = useState("");
  const [softwareVersion, setSoftwareVersion] = useState("V1.0");

  const [previewContent, setPreviewContent] = useState("请点击左侧开始扫描按钮，完毕后在目录树中勾选需要清洗的文件，然后点击本页右下角的 [执行深度清洗抽取] 等待结果...");
  const [previewLines, setPreviewLines] = useState(0);
  const [previewPages, setPreviewPages] = useState(0);
  const [linesPerPage, setLinesPerPage] = useState(0);

  // 统计信息
  const selectedCount = selectedIds.size;
  const originalLines = useMemo(() => files.filter(f => selectedIds.has(f.id)).reduce((acc, f) => acc + f.lines, 0), [files, selectedIds]);

  async function handleSelectDir() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (selected) {
        setRootPath(selected as string);
      }
    } catch (e) {
      console.error("Open dialog error:", e);
    }
  }

  async function handleScan() {
    if (!rootPath) {
      alert("请输入或使用文件夹图标选择扫描目录！");
      return;
    }
    setIsScanning(true);
    try {
      const result: any = await invoke("scan_project", {
        root: rootPath,
        customExcludes: excludeRules,
        extensions: selectedExtensions
      });
      // 更新文件树
      setFiles(result.files);

      const gitignoreRules = Array.isArray(result.gitignore_rules)
        ? result.gitignore_rules.filter((rule: unknown): rule is string => typeof rule === "string" && rule.trim().length > 0)
        : [];
      if (gitignoreRules.length > 0) {
        setExcludeRules((prev) => {
          const merged = [...prev];
          for (const rule of gitignoreRules) {
            if (!merged.includes(rule)) {
              merged.push(rule);
            }
          }
          return merged;
        });
      }

      // 动态将本次真实扫描到的所有代码后缀更新至界面的匹配列表中
      const foundExts = Object.keys(result.language_counts);
      setSelectedExtensions(foundExts);

      // 默认将扫描出来的文件全部设置为选中
      const allIds = new Set<number>(result.files.map((f: FileInfo) => f.id));
      setSelectedIds(allIds);
      const gitignoreMsg = gitignoreRules.length > 0
        ? `\n\n已自动读取 .gitignore 规则: ${gitignoreRules.length} 条。`
        : "";
      setPreviewContent(`✅ 扫描结束！\n\n一共发现核心代码文件: ${result.total_files} 个。${gitignoreMsg}\n\n您可以在中间的「项目结构」树状图中展开各个层级，按需剔除不需要申报的文件（例如测试框架代码、编译配置脚本等）。\n\n配置完毕后，请点击右下角的执行深度提纯。`);
      setPreviewLines(0);
      setPreviewPages(0);
      setLinesPerPage(0);
    } catch (e) {
      alert("扫描失败: " + e);
    } finally {
      setIsScanning(false);
    }
  }

  async function handleExtract() {
    const selectedFilePaths = files.filter(f => selectedIds.has(f.id)).map(f => f.absolute_path);
    if (selectedFilePaths.length === 0) {
      alert("请至少在结构树中勾选一个文件进行清洗！");
      return;
    }

    setIsExtracting(true);
    setPreviewContent("深度代码清洗执行中，请耐心等待...");
    try {
      const res: any = await invoke("execute_extraction", {
        files: selectedFilePaths,
        config: {
          remove_comments: true,
          compact_lines: true
        }
      });
      setPreviewContent(res.content);
      setPreviewLines(res.line_count);
      setPreviewPages(res.page_count);
      setLinesPerPage(res.lines_per_page);
    } catch (e) {
      alert("提取失败：" + e);
    } finally {
      setIsExtracting(false);
    }
  }

  function handleAddRule() {
    if (newRule && !excludeRules.includes(newRule)) {
      setExcludeRules([...excludeRules, newRule]);
      setNewRule("");
    }
  }

  function handleRemoveRule(index: number) {
    setExcludeRules(excludeRules.filter((_, i) => i !== index));
  }

  async function handleExportWord() {
    if (!previewContent) return;
    try {
      const documentTitle = buildDocumentTitle(softwareName, softwareVersion);
      const savePath = await save({
        filters: [{
          name: 'Word 文档',
          extensions: ['docx']
        }],
        defaultPath: `${sanitizeWindowsFileName(documentTitle)}.docx`,
      });

      if (savePath) {
        setIsExtracting(true); // 复用 extracting 状态展现加载等待图标
        const msg: string = await invoke("export_to_docx", {
          content: previewContent,
          config: {
            software_name: softwareName.trim(),
            software_version: softwareVersion.trim() || "V1.0",
            save_path: savePath
          }
        });
        alert(msg);
      }
    } catch (e) {
      alert(`导出遇到错误: ${e}`);
    } finally {
      setIsExtracting(false);
    }
  }

  return (
    <Box sx={{ flexGrow: 1, height: '100vh', display: 'flex', flexDirection: 'column', bgcolor: '#f4f6f8' }}>

      {/* 顶部通栏 Dashboard */}
      <Paper elevation={1} sx={{ m: 2, p: 2, display: 'flex', justifyContent: 'space-around', alignItems: 'center', flexShrink: 0 }}>
        <Box textAlign="center">
          <Typography variant="caption" color="text.secondary">选中文件</Typography>
          <Typography variant="h5" fontWeight="bold" color="primary">{selectedCount} <Typography variant="caption" color="text.secondary">/ {files.length}</Typography></Typography>
        </Box>
        <Box textAlign="center">
          <Typography variant="caption" color="text.secondary">累计原始行数</Typography>
          <Typography variant="h5" fontWeight="bold">{originalLines}</Typography>
        </Box>
        <Box textAlign="center">
          <Typography variant="caption" color="text.secondary">深度清洗后净行数</Typography>
          <Typography variant="h5" fontWeight="bold" color="success.main">{previewLines}</Typography>
        </Box>
        <Box textAlign="center">
          <Typography variant="caption" color="text.secondary">Word 页数 (每页最多 {linesPerPage || "-"} 行，最多60页)</Typography>
          <Typography variant="h5" fontWeight="bold" color="primary.main">{previewPages}</Typography>
        </Box>
      </Paper>

      {/* 预留高度的进度条包裹层，避免闪烁 */}
      <Box sx={{ height: 4, mx: 2, mb: 2 }}>
        {(isScanning || isExtracting) && (
          <LinearProgress sx={{ borderRadius: 2 }} />
        )}
      </Box>

      {/* 绝对控制的 Flex 极限四栏区域 */}
      <Box sx={{ display: 'flex', flexDirection: 'row', gap: 2, px: 2, pb: 2, flex: 1, overflow: 'hidden' }}>

        {/* 第1栏：配置与规则 (固定 280px) */}
        <Box sx={{ width: 280, display: 'flex', flexDirection: 'column', flexShrink: 0 }}>
          <Paper sx={{ p: 2, mb: 2 }}>
            <Typography variant="subtitle2" fontWeight="bold" mb={2}>1. 扫描配置</Typography>
            <Stack direction="row" spacing={1} mb={2}>
              <TextField
                size="small"
                placeholder="根目录绝对路径"
                value={rootPath}
                onChange={(e) => setRootPath(e.target.value)}
                fullWidth
              />
              <Button variant="outlined" sx={{ minWidth: '40px' }} onClick={handleSelectDir}>
                <FolderOpenIcon />
              </Button>
            </Stack>
            <Button
              variant="contained"
              startIcon={isScanning ? <CircularProgress size={20} color="inherit" /> : <SearchIcon />}
              fullWidth
              onClick={handleScan}
              disabled={isScanning}
            >
              {isScanning ? "目录扫描执行中..." : "👉 开始目录扫描"}
            </Button>
          </Paper>

          <Paper sx={{ p: 2, flexGrow: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <Typography variant="subtitle2" fontWeight="bold" mb={2}>2. 目标扩展名 (支持回车录入)</Typography>
            <Autocomplete
              multiple
              freeSolo
              size="small"
              options={[]}
              value={selectedExtensions}
              onChange={(_, newVal) => setSelectedExtensions(newVal)}
              renderTags={(value: readonly string[], getTagProps) =>
                value.map((option: string, index: number) => {
                  const { key, ...tagProps } = getTagProps({ index });
                  return <Chip variant="outlined" size="small" label={option} key={key} {...tagProps} />;
                })
              }
              renderInput={(params) => (
                <TextField {...params} placeholder="输入后缀如 js, rs..." />
              )}
              sx={{ mb: 3 }}
            />

            <Typography variant="subtitle2" fontWeight="bold" mb={2}>3. 排除匹配规则 (正则表达式)</Typography>
            <Stack direction="row" spacing={1} mb={2}>
              <TextField
                size="small"
                placeholder="正则表达式如 \.git$"
                value={newRule}
                onChange={(e) => setNewRule(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleAddRule()}
                fullWidth
              />
              <Button variant="contained" sx={{ minWidth: '40px', px: 1 }} onClick={handleAddRule}>┼</Button>
            </Stack>
            <Box sx={{ flexGrow: 1, overflowY: 'auto', pr: 1 }}>
              <Stack spacing={1}>
                {excludeRules.map((rule, idx) => (
                  <Paper key={idx} variant="outlined" sx={{ px: 1.5, py: 0.5, display: 'flex', alignItems: 'center', justifyContent: 'space-between', bgcolor: '#fafafa' }}>
                    <Typography variant="body2" sx={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {rule}
                    </Typography>
                    <IconButton size="small" onClick={() => handleRemoveRule(idx)}>
                      <DeleteIcon fontSize="small" color="action" />
                    </IconButton>
                  </Paper>
                ))}
              </Stack>
            </Box>
          </Paper>
        </Box>

        {/* 第2栏：项目结构树 (固定 320px) */}
        <Box sx={{ width: 320, display: 'flex', flexDirection: 'column', flexShrink: 0 }}>
          <Paper sx={{ p: 2, height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <Typography variant="subtitle2" fontWeight="bold" mb={2}>4. 项目结构 (联动勾选子文件)</Typography>
            <Box sx={{ flexGrow: 1, overflowX: 'auto', overflowY: 'auto', border: '1px solid #e0e0e0', borderRadius: 1 }}>
              <FileTree files={files} selectedIds={selectedIds} onSelectionChange={setSelectedIds} />
            </Box>
          </Paper>
        </Box>

        {/* 第3栏：预览大区 (自动伸缩拉满整个中部 flex: 1) */}
        <Box sx={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 200 }}>
          <Paper sx={{ p: 2, height: '100%', display: 'flex', flexDirection: 'column' }}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
              <Typography variant="subtitle2" fontWeight="bold">5. 实时清洗代码大屏</Typography>
              <Button
                variant="contained"
                color="secondary"
                size="small"
                disabled={previewLines === 0 || isExtracting}
                onClick={handleExportWord}
              >
                💾 一键生成软著标准文档 (.docx)
              </Button>
            </Box>
            <Box sx={{ flexGrow: 1, bgcolor: '#1e1e1e', color: '#d4d4d4', p: 2, borderRadius: 1, overflowY: 'auto', overflowX: 'hidden', fontSize: '13px' }}>
              <pre style={{ margin: 0, whiteSpace: 'pre-wrap', fontFamily: 'Consolas, monospace' }}>
                {previewContent}
              </pre>
            </Box>
          </Paper>
        </Box>

        {/* 第4栏：参数与操作 (固定 280px) */}
        <Box sx={{ width: 280, display: 'flex', flexDirection: 'column', flexShrink: 0 }}>
          <Paper sx={{ p: 2, flexGrow: 1, display: 'flex', flexDirection: 'column' }}>
            <Typography variant="subtitle2" fontWeight="bold" mb={2}>6. 深度清洗策略</Typography>
            <Stack spacing={0} mb={3}>
              <Stack direction="row" alignItems="center"><Checkbox size="small" defaultChecked /><Typography variant="body2">去除所有代码注释</Typography></Stack>
              <Stack direction="row" alignItems="center"><Checkbox size="small" defaultChecked /><Typography variant="body2">去除多余空行</Typography></Stack>
            </Stack>

            <Typography variant="subtitle2" fontWeight="bold" mb={2}>7. 申报元数据</Typography>
            <Stack spacing={2} mb={3}>
              <TextField size="small" label="软件全称" placeholder="必填" value={softwareName} onChange={e => setSoftwareName(e.target.value)} fullWidth />
              <TextField size="small" label="版本号" value={softwareVersion} onChange={e => setSoftwareVersion(e.target.value)} fullWidth />
            </Stack>

            <Box mt="auto">
              <Button
                variant="contained"
                color="secondary"
                fullWidth
                sx={{ mb: 2, py: 1.5, fontWeight: 'bold' }}
                onClick={handleExtract}
                disabled={isExtracting}
              >
                {isExtracting ? "正在剔除杂项..." : "🚀 执行深度清洗提取"}
              </Button>
            </Box>
          </Paper>
        </Box>

      </Box>
    </Box>
  );
}

export default App;
