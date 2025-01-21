import { Routes, Route } from "react-router";
import Home from "./pages/Home";
import NewSymphony from "./pages/NewSymphony";
import EditSymphony from "./pages/EditSymphony";
import NewNote from "./pages/NewNote";
import EditNote from "./pages/EditNote";

function App() {
  return (
    <div className="container mx-auto p-4">
      <h1 className="text-3xl font-bold mb-4">Symphony Manager</h1>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/new-symphony" element={<NewSymphony />} />
        <Route path="/edit-symphony/:id" element={<EditSymphony />} />
        <Route path="/new-note/:symphonyId" element={<NewNote />} />
        <Route path="/edit-note/:symphonyId/:noteId" element={<EditNote />} />
      </Routes>
    </div>
  );
}

export default App;
