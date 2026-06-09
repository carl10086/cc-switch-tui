import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Link, NavLink, Route, Routes } from 'react-router-dom';
import { useHealth } from './api/hooks';
import { apiGet } from './api/client';
import type { Template } from './api/types';
import { ThemeToggle } from './components/ThemeToggle';
import { TraceDashboard } from './pages/traces/Dashboard';
import { ApplyPage } from './routes/ApplyPage';
import { ConfigPage } from './routes/ConfigPage';
import { InstanceDetailPage } from './routes/InstanceDetailPage';
import { InstancesPage } from './routes/InstancesPage';
import { SettingsPage } from './routes/SettingsPage';

export default function App() {
  const { data: health } = useHealth();
  const qc = useQueryClient();

  // 启动时一次性预取 templates，让创建/编辑/Apply 打开时直接命中缓存。
  useEffect(() => {
    void qc.prefetchQuery({
      queryKey: ['templates'],
      queryFn: () => apiGet<Template[]>('/api/templates'),
      staleTime: 60_000,
    });
  }, [qc]);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border">
        <div className="max-w-6xl mx-auto px-6 py-3 flex items-center justify-between">
          <Link to="/" className="text-lg font-semibold">
            cc-switch
          </Link>
          <div className="flex items-center gap-3 text-xs">
            {health && (
              <span
                className="text-muted-foreground"
                title={`v${health.version} · db: ${health.dbPath}`}
              >
                <span className="inline-block w-1.5 h-1.5 rounded-full bg-green-500 mr-1" />
                {health.status}
              </span>
            )}
            <ThemeToggle />
          </div>
        </div>
      </header>

      <nav className="border-b border-border">
        <div className="max-w-6xl mx-auto px-6 flex gap-6 text-sm">
          <StyledNavLink to="/">Instances</StyledNavLink>
          <span className="border-l border-border" />
          <StyledNavLink to="/apply">Apply</StyledNavLink>
          <span className="border-l border-border" />
          <StyledNavLink to="/config">Config</StyledNavLink>
          <StyledNavLink to="/traces">Traces</StyledNavLink>
          <StyledNavLink to="/settings">Settings</StyledNavLink>
        </div>
      </nav>

      <main className="max-w-6xl mx-auto px-6 py-6">
        <Routes>
          <Route path="/" element={<InstancesPage />} />
          <Route path="/instances/:id" element={<InstanceDetailPage />} />
          <Route path="/apply" element={<ApplyPage />} />
          <Route path="/config" element={<ConfigPage />} />
          <Route path="/traces" element={<TraceDashboard />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
}

function StyledNavLink({ to, children }: { to: string; children: React.ReactNode }) {
  return (
    <NavLink
      to={to}
      end
      className={({ isActive }) =>
        `py-2 border-b-2 ${
          isActive
            ? 'border-primary text-foreground'
            : 'border-transparent text-muted-foreground hover:text-foreground hover:border-primary'
        }`
      }
    >
      {children}
    </NavLink>
  );
}
