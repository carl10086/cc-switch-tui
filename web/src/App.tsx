import { Link, Route, Routes } from 'react-router-dom';
import { useHealth } from './api/hooks';
import { ThemeToggle } from './components/ThemeToggle';
import { InstanceDetailPage } from './routes/InstanceDetailPage';
import { InstancesPage } from './routes/InstancesPage';

export default function App() {
  const { data: health } = useHealth();

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
          <NavLink to="/">Instances</NavLink>
        </div>
      </nav>

      <main className="max-w-6xl mx-auto px-6 py-6">
        <Routes>
          <Route path="/" element={<InstancesPage />} />
          <Route path="/instances/:id" element={<InstanceDetailPage />} />
        </Routes>
      </main>
    </div>
  );
}

function NavLink({ to, children }: { to: string; children: React.ReactNode }) {
  return (
    <Link
      to={to}
      className="py-2 border-b-2 border-transparent hover:border-primary text-muted-foreground hover:text-foreground"
    >
      {children}
    </Link>
  );
}
