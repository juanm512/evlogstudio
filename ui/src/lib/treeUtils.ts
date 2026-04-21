import type { SchemaField } from './types';
import { browser } from '$app/environment';

export interface TreeNode {
  key: string;
  fullPath: string | null;
  children: TreeNode[];
  fieldType: string | null;
  expanded: boolean;
}

export interface TreeOptions {
  excludeFields?: string[];
  expandLevel1?: boolean;
}

export function buildTree(
  fields: SchemaField[],
  storageKey: string,
  options: TreeOptions = {}
): TreeNode[] {
  const root: TreeNode[] = [];
  const savedState = getSavedExpansionState(storageKey);
  const excludeFields = options.excludeFields || [];
  const expandLevel1 = options.expandLevel1 ?? true;

  fields.forEach(field => {
    if (excludeFields.includes(field.field_path)) return;

    const segments = field.field_path.split('.');
    let currentLevel = root;
    let pathSoFar = '';

    segments.forEach((segment, index) => {
      pathSoFar = pathSoFar ? `${pathSoFar}.${segment}` : segment;
      const isLast = index === segments.length - 1;
      
      let node = currentLevel.find(n => n.key === segment);
      if (!node) {
        node = {
          key: segment,
          fullPath: isLast ? field.field_path : null,
          children: [],
          fieldType: isLast ? field.field_type : null,
          expanded: savedState[pathSoFar] ?? (expandLevel1 && index === 0)
        };
        currentLevel.push(node);
      }
      currentLevel = node.children;
    });
  });

  sortTree(root);
  return root;
}

export function sortTree(nodes: TreeNode[]) {
  nodes.sort((a, b) => {
    // Leaves before branches
    const aIsLeaf = a.children.length === 0;
    const bIsLeaf = b.children.length === 0;
    if (aIsLeaf !== bIsLeaf) return aIsLeaf ? -1 : 1;
    return a.key.localeCompare(b.key);
  });
  nodes.forEach(node => {
    if (node.children.length > 0) sortTree(node.children);
  });
}

export function filterTree(nodes: TreeNode[], query: string): TreeNode[] {
  if (!query) return nodes;
  const q = query.toLowerCase();

  return nodes.reduce((acc: TreeNode[], node) => {
    const matches = node.fullPath?.toLowerCase().includes(q) || node.key.toLowerCase().includes(q);
    
    const filteredChildren = filterTree(node.children, query);
    const hasMatchingChild = filteredChildren.length > 0;

    if (matches || hasMatchingChild) {
      acc.push({
        ...node,
        children: filteredChildren,
        // Expand matching branches automatically
        expanded: hasMatchingChild ? true : node.expanded
      });
    }
    return acc;
  }, []);
}

export function getSavedExpansionState(storageKey: string): Record<string, boolean> {
  if (!browser) return {};
  try {
    const saved = localStorage.getItem(storageKey);
    return saved ? JSON.parse(saved) : {};
  } catch {
    return {};
  }
}

export function saveExpansionState(storageKey: string, path: string, expanded: boolean) {
  if (!browser) return;
  try {
    const saved = localStorage.getItem(storageKey);
    const state = saved ? JSON.parse(saved) : {};
    state[path] = expanded;
    localStorage.setItem(storageKey, JSON.stringify(state));
  } catch { /* ignore */ }
}

export function countSelectedLeaves(node: TreeNode, selectedPaths: string[]): number {
  if (node.children.length === 0) {
    return selectedPaths.includes(node.fullPath!) ? 1 : 0;
  }
  return node.children.reduce((sum, child) => sum + countSelectedLeaves(child, selectedPaths), 0);
}
