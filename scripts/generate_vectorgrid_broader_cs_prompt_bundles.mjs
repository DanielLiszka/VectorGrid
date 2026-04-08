import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const sassResearchDir = path.join(repoRoot, "SASS research");
const packRelativePath = path.join(
  "SASS research",
  "vectorgrid_broader_cs_research_question_pack_2026-04-05.md",
);
const packPath = path.join(repoRoot, packRelativePath);

const questionConfigs = [
  { number: 1, folderName: "Question1", sectionPrefix: "Question 1:" },
  { number: 2, folderName: "Question2", sectionPrefix: "Question 2:" },
  { number: 3, folderName: "Question3", sectionPrefix: "Question 3:" },
  { number: 4, folderName: "Question4", sectionPrefix: "Question 4:" },
  { number: 5, folderName: "Question5", sectionPrefix: "Question 5:" },
  { number: 6, folderName: "Question6", sectionPrefix: "Question 6:" },
  { number: 7, folderName: "Question7", sectionPrefix: "Question 7:" },
  { number: 8, folderName: "Question8", sectionPrefix: "Question 8:" },
];

function normalizeMarkdown(text) {
  return text.replace(/\r\n/g, "\n");
}

function parseTopLevelSections(markdown) {
  const matches = [...markdown.matchAll(/^## (.+)$/gm)];
  const sections = new Map();

  for (let index = 0; index < matches.length; index += 1) {
    const match = matches[index];
    const title = match[1].trim();
    const bodyStart = (match.index ?? 0) + match[0].length;
    const bodyEnd = index + 1 < matches.length ? matches[index + 1].index ?? markdown.length : markdown.length;
    const body = markdown.slice(bodyStart, bodyEnd).trim();
    sections.set(title, body);
  }

  return sections;
}

function getSection(sections, title) {
  const body = sections.get(title);
  if (!body) {
    throw new Error(`Missing section: ${title}`);
  }

  return { title, body };
}

function getSectionByPrefix(sections, prefix) {
  for (const [title, body] of sections.entries()) {
    if (title.startsWith(prefix)) {
      return { title, body };
    }
  }

  throw new Error(`Missing section prefix: ${prefix}`);
}

function renderSection(section) {
  return `## ${section.title}\n\n${section.body.trim()}\n`;
}

function extractBacktickBullets(markdown) {
  return [...markdown.matchAll(/^- `([^`]+)`$/gm)].map((match) => match[1].trim());
}

function extractQuestionAttachments(questionBody) {
  const match = questionBody.match(/Attach in addition:\n\n([\s\S]*?)\nPrompt:\n/);
  if (!match) {
    return [];
  }

  return extractBacktickBullets(match[1]);
}

function uniquePaths(paths) {
  return [...new Set(paths)];
}

function normalizeRelativePath(relPath) {
  return relPath.replace(/\\/g, "/");
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function writeTextFile(targetPath, content) {
  ensureDir(path.dirname(targetPath));
  fs.writeFileSync(targetPath, content.replace(/\n/g, "\r\n"), "utf8");
}

function copyMirroredFile(rootTargetDir, relPath) {
  const normalizedRelPath = normalizeRelativePath(relPath);
  const sourcePath = path.join(repoRoot, normalizedRelPath);
  const targetPath = path.join(rootTargetDir, normalizedRelPath);

  if (!fs.existsSync(sourcePath)) {
    throw new Error(`Missing source file: ${normalizedRelPath}`);
  }

  ensureDir(path.dirname(targetPath));
  fs.copyFileSync(sourcePath, targetPath);
}

function renderPathList(paths) {
  return paths.map((filePath) => `- \`${normalizeRelativePath(filePath)}\``).join("\n");
}

function buildReadme({
  folderName,
  questionTitle,
  sharedFiles,
  questionSpecificFiles,
  allFiles,
}) {
  return `# ${folderName} Upload Bundle

Source pack: \`${normalizeRelativePath(packRelativePath)}\`

Question: \`${questionTitle}\`

This folder is a self-contained upload bundle for GPT-5.4 Pro / Research Mode.

## What Is Included

- \`prompts/01_first_pass_prompt.md\`
- \`prompts/02_second_pass_prompt.md\`
- \`prompts/03_final_consolidation_prompt.md\`
- \`files/\`, which mirrors the original repo-relative paths for all copied attachments

## Recommended Use Order

1. Attach the copied files under \`files/\` and use \`prompts/01_first_pass_prompt.md\`.
2. After the first answer, reuse the same files and use \`prompts/02_second_pass_prompt.md\`.
3. After the second answer, reuse the same files and use \`prompts/03_final_consolidation_prompt.md\`.

## Shared Baseline Files

${renderPathList(sharedFiles)}

## Question-Specific Files

${questionSpecificFiles.length > 0 ? renderPathList(questionSpecificFiles) : "- None beyond the shared baseline set"}

## Full Copied File Set

${renderPathList(allFiles)}
`;
}

function buildFirstPassPrompt({
  questionTitle,
  allFiles,
  sections,
  questionSection,
}) {
  const orderedSections = [
    getSection(sections, "Core Conclusion Before You Spend Research Mode"),
    getSection(sections, "Novelty Bar"),
    getSection(sections, "How To Treat A Custom CUDA Implementation"),
    getSection(sections, "What Research Mode Should Always Be Forced To Answer"),
    getSection(sections, "Shared Answer Shape"),
    getSection(sections, "First-Pass Prompt Discipline"),
  ];

  return `# First-Pass Prompt

Question: \`${questionTitle}\`

Attach the copied files from this bundle that mirror these original repo-relative paths:

${renderPathList(allFiles)}

Use the question-specific prompt below together with the shared guidance from the source pack.

${orderedSections.map(renderSection).join("\n")}
## ${questionSection.title}

${questionSection.body.trim()}
`;
}

function buildSecondPassPrompt({
  questionTitle,
  allFiles,
  sections,
}) {
  const orderedSections = [
    getSection(sections, "Second-Pass Prompt Discipline"),
    getSection(sections, "Revised Priority Order After Literature Search"),
    getSection(sections, "Reusable Second-Pass Prompt Template"),
  ];

  return `# Second-Pass Prompt

Question: \`${questionTitle}\`

Reuse the same copied attachment set from the first pass:

${renderPathList(allFiles)}

Use this only after the first-pass answer exists.

${orderedSections.map(renderSection).join("\n")}`;
}

function buildFinalConsolidationPrompt({
  questionTitle,
  allFiles,
  sections,
}) {
  const orderedSections = [
    getSection(sections, "Final Consolidation Prompt Discipline"),
    getSection(sections, "Mandatory Baseline Matrix"),
    getSection(sections, "Candidate Benchmark Slate Inside This Repo"),
    getSection(sections, "Go / No-Go Rubric"),
    getSection(sections, "Publishable Negative Results"),
    getSection(sections, "Reusable Final Consolidation Prompt Template"),
  ];

  return `# Final Consolidation Prompt

Question: \`${questionTitle}\`

Reuse the same copied attachment set from the first and second passes:

${renderPathList(allFiles)}

Use this only after the second-pass answer exists.

${orderedSections.map(renderSection).join("\n")}`;
}

function buildTopLevelIndex(entries) {
  const generatedDate = new Date().toISOString().slice(0, 10);

  return `# VectorGrid Broader CS Prompt Bundles

Generated: ${generatedDate}

Source pack: \`${normalizeRelativePath(packRelativePath)}\`

These folders package the broader CS research question pack into self-contained upload bundles. Each question folder contains:

- prompt files for the first pass, second pass, and final consolidation pass
- a \`files/\` directory mirroring the original repo-relative paths for every shared baseline file and question-specific attachment

## Question Folders

${entries
  .map(
    (entry) =>
      `${entry.number}. \`${entry.folderName}\` - \`${entry.questionTitle}\` (${entry.fileCount} copied files)`,
  )
  .join("\n")}

## Prompt Flow

1. Start with \`prompts/01_first_pass_prompt.md\` inside the question folder.
2. Reuse the same attachment set with \`prompts/02_second_pass_prompt.md\`.
3. Reuse the same attachment set again with \`prompts/03_final_consolidation_prompt.md\`.

## Regeneration

Rebuild these bundles with:

\`node scripts/generate_vectorgrid_broader_cs_prompt_bundles.mjs\`
`;
}

const questionPack = normalizeMarkdown(fs.readFileSync(packPath, "utf8"));
const sections = parseTopLevelSections(questionPack);

const sharedBaselineSection = getSection(sections, "Shared Baseline Files");
const sharedBaselineFiles = extractBacktickBullets(sharedBaselineSection.body);

const indexEntries = [];

for (const config of questionConfigs) {
  const questionSection = getSectionByPrefix(sections, config.sectionPrefix);
  const questionSpecificFiles = uniquePaths(
    extractQuestionAttachments(questionSection.body).filter(
      (relPath) => !sharedBaselineFiles.includes(relPath),
    ),
  );
  const allFiles = uniquePaths([...sharedBaselineFiles, ...questionSpecificFiles]);

  const questionDir = path.join(sassResearchDir, config.folderName);
  const promptsDir = path.join(questionDir, "prompts");
  const filesDir = path.join(questionDir, "files");

  ensureDir(promptsDir);
  ensureDir(filesDir);

  for (const relPath of allFiles) {
    copyMirroredFile(filesDir, relPath);
  }

  writeTextFile(
    path.join(questionDir, "README.md"),
    buildReadme({
      folderName: config.folderName,
      questionTitle: questionSection.title,
      sharedFiles: sharedBaselineFiles,
      questionSpecificFiles,
      allFiles,
    }),
  );

  writeTextFile(
    path.join(promptsDir, "01_first_pass_prompt.md"),
    buildFirstPassPrompt({
      questionTitle: questionSection.title,
      allFiles,
      sections,
      questionSection,
    }),
  );

  writeTextFile(
    path.join(promptsDir, "02_second_pass_prompt.md"),
    buildSecondPassPrompt({
      questionTitle: questionSection.title,
      allFiles,
      sections,
    }),
  );

  writeTextFile(
    path.join(promptsDir, "03_final_consolidation_prompt.md"),
    buildFinalConsolidationPrompt({
      questionTitle: questionSection.title,
      allFiles,
      sections,
    }),
  );

  indexEntries.push({
    number: config.number,
    folderName: config.folderName,
    questionTitle: questionSection.title,
    fileCount: allFiles.length,
  });
}

writeTextFile(
  path.join(sassResearchDir, "vectorgrid_broader_cs_prompt_bundle_index_2026-04-08.md"),
  buildTopLevelIndex(indexEntries),
);
