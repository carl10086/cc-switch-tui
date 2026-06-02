import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from './ui/table';
import type { Instance } from '../api/types';
import { useDeleteInstance } from '../api/hooks';
import { ConfirmDialog } from './ConfirmDialog';

export function InstancesTable({ instances }: { instances: Instance[] }) {
  const navigate = useNavigate();
  const deleteInst = useDeleteInstance();
  const [pending, setPending] = useState<Instance | null>(null);

  if (instances.length === 0) {
    return (
      <div className="text-muted-foreground p-8 text-center border border-dashed border-border rounded">
        No instances match. Adjust your search or click "New" to create one.
      </div>
    );
  }

  return (
    <>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Alias</TableHead>
            <TableHead>Template</TableHead>
            <TableHead>Model</TableHead>
            <TableHead>OpenCode Model</TableHead>
            <TableHead className="text-right">Flags</TableHead>
            <TableHead className="w-12 text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {instances.map((i) => (
            <TableRow
              key={i.id}
              onClick={() => navigate(`/instances/${i.id}`)}
              className="cursor-pointer hover:bg-muted/50 transition-colors"
              data-testid={`instance-row-${i.alias}`}
            >
              <TableCell className="font-mono">{i.alias}</TableCell>
              <TableCell>
                <span className="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
                  {i.templateId}
                </span>
              </TableCell>
              <TableCell className="font-mono text-xs">{i.modelId}</TableCell>
              <TableCell className="font-mono text-xs text-muted-foreground">
                {i.opencodeModelId || <span className="italic">none</span>}
              </TableCell>
              <TableCell className="text-right">
                {i.kvCacheEnabled && (
                  <span
                    className="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground"
                    title="KV Cache optimized"
                  >
                    KV
                  </span>
                )}
              </TableCell>
              <TableCell className="text-right">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    setPending(i);
                  }}
                  className="p-1 text-red-600 hover:bg-red-50 dark:hover:bg-red-950 rounded"
                  title="Delete instance"
                  aria-label={`Delete ${i.alias}`}
                >
                  🗑
                </button>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>

      <ConfirmDialog
        open={!!pending}
        title="Delete instance?"
        message={`This will permanently delete "${pending?.alias}".\nThis cannot be undone.`}
        confirmLabel="Delete"
        destructive
        onConfirm={async () => {
          if (!pending) return;
          await deleteInst.mutateAsync(pending.id);
          setPending(null);
        }}
        onCancel={() => setPending(null)}
      />
    </>
  );
}
