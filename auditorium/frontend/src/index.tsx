import { Route, Switch } from "wouter";
import Home from "./pages/Home";
import NewSymphony from "./pages/NewSymphony";
import EditSymphony from "./pages/EditSymphony";
import NewNote from "./pages/NewNote";
import EditNote from "./pages/EditNote";
import "./index.css";

export default function App() {
  return (
    <div className="container mx-auto p-4">
      <h1 className="text-3xl font-bold mb-4">Symphony Manager</h1>
      <Switch>
        <Route path="/" component={Home} />
        <Route path="/new-symphony" component={NewSymphony} />
        <Route path="/edit-symphony/:id" component={EditSymphony} />
        <Route path="/new-note/:symphonyId" component={NewNote} />
        <Route path="/edit-note/:symphonyId/:noteId" component={EditNote} />
      </Switch>
    </div>
  );
}
