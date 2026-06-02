import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from './ui/table';
import type { Instance } from '../api/types';

export function InstancesTable({
  instances,
  onRowClick,
}: {
  instances: Instance[];
  onRowClick?: (instance: Instance) => void;
}) {
  if (instances.length === 0) {
    return (
      <div className="text-muted-foreground p-8 text-center border border-dashed border-border rounded">
        No instances match. Adjust your search or click "New" to create one.
      </div>
    );
  }
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Alias</TableHead>
          <TableHead>Template</TableHead>
          <TableHead>Model</TableHead>
          <TableHead>OpenCode Model</TableHead>
          <TableHead className="text-right">Flags</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {instances.map((i) => (
          <TableRow
            key={i.id}
            onClick={onRowClick ? () => onRowClick(i) : undefined}
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
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
