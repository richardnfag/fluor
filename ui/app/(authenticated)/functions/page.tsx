"use client";

import { useEffect, useState } from "react";
import { Function, Trigger } from "@/types";
import {
  PrimaryButton,
  GhostButton,
  SecondaryButton,
} from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";

import { Modal } from "@/components/ui/Modal";
import { Card } from "@/components/ui/Card";
import { Pencil, Trash2, Plus, Play } from "lucide-react";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

export default function FunctionsPage() {
  const [functions, setFunctions] = useState<Function[]>([]);
  const [triggers, setTriggers] = useState<Trigger[]>([]); // All triggers
  const [selectedFunction, setSelectedFunction] = useState<Function | null>(
    null,
  );
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isCreateMode, setIsCreateMode] = useState(false);

  // Invoke state
  const [isInvokeModalOpen, setIsInvokeModalOpen] = useState(false);
  const [invokeInput, setInvokeInput] = useState("");
  const [invokeResult, setInvokeResult] = useState<string | null>(null);
  const [availableRoutes, setAvailableRoutes] = useState<Trigger[]>([]);
  const [selectedRoute, setSelectedRoute] = useState<string>("manual"); // 'manual' or trigger name

  useEffect(() => {
    fetchFunctions();
    fetchTriggers();
  }, []);

  const fetchFunctions = () => {
    fetch(`${API_URL}/functions`)
      .then((res) => res.json())
      .then((data) => setFunctions(data));
  };

  const fetchTriggers = () => {
    fetch(`${API_URL}/triggers`)
      .then((res) => res.json())
      .then((data) => setTriggers(data));
  };

  const handleEdit = (func: Function) => {
    setSelectedFunction({ ...func });
    setSelectedFile(null);
    setIsCreateMode(false);
    setIsModalOpen(true);
  };

  const handleCreate = () => {
    setSelectedFunction({
      name: "",
      language: "python",
      executable: "",
      cpu: "100m",
      memory: "128Mi",
    });
    setSelectedFile(null);
    setIsCreateMode(true);
    setIsModalOpen(true);
  };

  const handleUpdate = async () => {
    if (!selectedFunction) return;

    try {
      const url = isCreateMode
        ? `${API_URL}/functions`
        : `${API_URL}/functions/${selectedFunction.name}`;

      const method = isCreateMode ? "POST" : "PUT";

      let body: string | FormData;
      let headers: Record<string, string> = {};

      if (selectedFile) {
        const formData = new FormData();
        // Send function metadata as JSON string in 'function' field
        formData.append("function", JSON.stringify(selectedFunction));
        formData.append("file", selectedFile);
        body = formData;
        // Content-Type header is set automatically by browser for FormData
      } else {
        body = JSON.stringify(selectedFunction);
        headers["Content-Type"] = "application/json";
      }

      const res = await fetch(url, {
        method: method,
        headers: headers,
        body: body,
      });

      if (!res.ok) {
        const err = await res.text();
        alert(`Failed to save function: ${err}`);
        return;
      }

      setIsModalOpen(false);
      fetchFunctions();
      setSelectedFile(null); // Reset file selection
    } catch (error) {
      console.error(error);
      alert("An error occurred");
    }
  };

  const handleDelete = async (name: string) => {
    if (confirm(`Are you sure you want to delete ${name}?`)) {
      await fetch(`${API_URL}/functions/${name}`, {
        method: "DELETE",
      });
      fetchFunctions();
    }
  };

  const handleInvokeOpen = (func: Function) => {
    setSelectedFunction({ ...func });
    setInvokeInput("");
    setInvokeResult(null);

    // Find triggers for this function
    const routes = triggers.filter((t) => t.function === func.name);
    setAvailableRoutes(routes);

    if (routes.length > 0) {
      setSelectedRoute(routes[0].name);
    } else {
      setSelectedRoute(""); // No route available
    }

    setIsInvokeModalOpen(true);
  };

  const executeInvoke = async () => {
    if (!selectedFunction || !selectedRoute) {
      setInvokeResult(
        "Error: No route selected. Please create a trigger for this function.",
      );
      return;
    }
    setInvokeResult("Running...");

    try {
      const route = availableRoutes.find((r) => r.name === selectedRoute);
      if (!route) {
        throw new Error("Route not found");
      }

      const url = `${API_URL}/function${route.path}`;
      const method = route.method;

      // Don't send body for GET/HEAD
      const options: RequestInit = {
        method: method,
      };

      if (method !== "GET" && method !== "HEAD") {
        options.body = invokeInput;
      }

      const res = await fetch(url, options);
      const text = await res.text();
      setInvokeResult(text);
    } catch (e) {
      setInvokeResult(`Error: ${e}`);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div className="flex flex-col gap-2">
          <h1 className="text-3xl font-bold tracking-tight">Functions</h1>
          <p className="text-[var(--accents-5)]">
            Manage your serverless functions.
          </p>
        </div>
        <PrimaryButton onClick={handleCreate}>
          <Plus size={16} className="mr-2" />
          New Function
        </PrimaryButton>
      </div>

      <Card className="overflow-hidden">
        <table className="w-full text-left text-sm">
          <thead className="border-b border-white/20 dark:border-white/10 bg-black/5 dark:bg-white/5">
            <tr>
              <th className="px-6 py-3 font-medium text-[var(--accents-5)]">
                Name
              </th>
              <th className="px-6 py-3 font-medium text-[var(--accents-5)]">
                Language
              </th>
              <th className="px-6 py-3 font-medium text-[var(--accents-5)]">
                Executable
              </th>
              <th className="px-6 py-3 font-medium text-[var(--accents-5)]">
                Actions
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-white/20 dark:divide-white/10">
            {functions.map((func) => (
              <tr
                key={func.name}
                className="hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
              >
                <td className="px-6 py-4 font-medium">{func.name}</td>
                <td className="px-6 py-4 text-[var(--accents-5)]">
                  {func.language}
                </td>
                <td className="px-6 py-4 font-mono text-xs text-[var(--accents-5)]">
                  {func.executable}
                </td>
                <td className="px-6 py-4">
                  <div className="flex gap-2">
                    <GhostButton
                      size="sm"
                      onClick={() =>
                        (window.location.href = `/functions/${func.name}`)
                      }
                      className="h-8 w-8 p-0"
                      title="View Details"
                    >
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      >
                        <path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0" />
                        <circle cx="12" cy="12" r="3" />
                      </svg>
                    </GhostButton>
                    <GhostButton
                      size="sm"
                      onClick={() => handleInvokeOpen(func)}
                      className="h-8 w-8 p-0 text-green-600 hover:bg-green-50 hover:text-green-700"
                      title="Invoke"
                    >
                      <Play size={16} />
                    </GhostButton>
                    <GhostButton
                      size="sm"
                      onClick={() => handleEdit(func)}
                      disabled={func.readonly}
                      className={`h-8 w-8 p-0 ${func.readonly ? "opacity-50 cursor-not-allowed text-[var(--accents-3)]" : ""}`}
                      title={func.readonly ? "Read-only" : "Edit"}
                    >
                      <Pencil size={16} />
                    </GhostButton>
                    <GhostButton
                      size="sm"
                      onClick={() => handleDelete(func.name)}
                      disabled={func.readonly}
                      className={`h-8 w-8 p-0 ${func.readonly ? "opacity-50 cursor-not-allowed text-[var(--accents-3)]" : "text-red-600 hover:bg-red-50 hover:text-red-700"}`}
                      title={func.readonly ? "Read-only" : "Delete"}
                    >
                      <Trash2 size={16} />
                    </GhostButton>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      {selectedFunction && (
        <Modal
          isOpen={isModalOpen}
          title={isCreateMode ? "Create Function" : "Function Details"}
          onClose={() => setIsModalOpen(false)}
        >
          <div className="space-y-4">
            <Input
              id="name"
              label="Name"
              value={selectedFunction.name}
              readOnly={!isCreateMode}
              onChange={(e) =>
                setSelectedFunction({
                  ...selectedFunction,
                  name: e.target.value,
                })
              }
            />
            <Select
              id="language"
              label="Language"
              value={selectedFunction.language}
              onChange={(e) => {
                const newLang = e.target.value as any;
                setSelectedFunction({
                  ...selectedFunction,
                  language: newLang,
                });
              }}
              options={[
                { value: "python", label: "Python" },
                { value: "rust", label: "Rust" },
                { value: "go", label: "Go" },
              ]}
            />
            <div className="space-y-1.5">
              <label
                htmlFor="file-upload"
                className="text-sm font-medium text-[var(--accents-5)]"
              >
                Wasm Binary
              </label>
              <Input
                id="file-upload"
                type="file"
                accept=".wasm"
                onChange={(e) => {
                  if (e.target.files && e.target.files[0]) {
                    setSelectedFile(e.target.files[0]);
                  }
                }}
              />
              {selectedFile && (
                <p className="text-xs text-green-500">
                  Selected: {selectedFile.name}
                </p>
              )}
            </div>

            <Input
              id="executable"
              label="Executable Path (Read Only / Server Managed)"
              value={selectedFunction.executable || ""}
              readOnly
              placeholder="(Auto-generated on upload)"
            />
            <div className="grid grid-cols-2 gap-4">
              <Input
                id="cpu"
                label="CPU"
                value={selectedFunction.cpu}
                onChange={(e) =>
                  setSelectedFunction({
                    ...selectedFunction,
                    cpu: e.target.value,
                  })
                }
              />
              <Input
                id="memory"
                label="Memory"
                value={selectedFunction.memory}
                onChange={(e) =>
                  setSelectedFunction({
                    ...selectedFunction,
                    memory: e.target.value,
                  })
                }
              />
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <SecondaryButton onClick={() => setIsModalOpen(false)}>
                Cancel
              </SecondaryButton>
              <PrimaryButton onClick={handleUpdate}>
                {isCreateMode ? "Create" : "Update"}
              </PrimaryButton>
            </div>
          </div>
        </Modal>
      )}

      {selectedFunction && (
        <Modal
          isOpen={isInvokeModalOpen}
          title={`Invoke ${selectedFunction.name}`}
          onClose={() => setIsInvokeModalOpen(false)}
        >
          <div className="space-y-4">
            {availableRoutes.length > 0 ? (
              <Select
                id="route"
                label="Route"
                value={selectedRoute}
                onChange={(e) => setSelectedRoute(e.target.value)}
                options={availableRoutes.map((r) => ({
                  value: r.name,
                  label: `${r.method} ${r.path} (${r.name})`,
                }))}
              />
            ) : (
              <div className="rounded-md bg-yellow-50 dark:bg-yellow-900/20 p-3">
                <p className="text-sm text-yellow-700 dark:text-yellow-400">
                  No triggers found for this function. Create a trigger to
                  invoke it.
                </p>
              </div>
            )}

            <div className="space-y-1.5">
              <label
                htmlFor="input"
                className="text-sm font-medium text-[var(--accents-5)]"
              >
                Input
              </label>
              <textarea
                id="input"
                value={invokeInput}
                onChange={(e) => setInvokeInput(e.target.value)}
                placeholder="Function input..."
                className="flex min-h-[100px] w-full rounded-md border border-white/20 dark:border-white/10 bg-white/5 dark:bg-black/5 backdrop-blur-sm px-3 py-2 text-sm font-mono placeholder:text-[var(--accents-3)] focus:outline-none focus:ring-2 focus:ring-white/20 focus:border-[var(--foreground)]"
              />
              {selectedRoute &&
                availableRoutes.find((r) => r.name === selectedRoute)
                  ?.method === "GET" && (
                  <p className="text-xs text-[var(--accents-5)]">
                    Input body is ignored for GET requests.
                  </p>
                )}
            </div>

            {invokeResult && (
              <div className="space-y-1.5">
                <label className="text-sm font-medium text-[var(--accents-5)]">
                  Result
                </label>
                <div className="rounded-md bg-black/5 dark:bg-white/5 p-3 text-sm font-mono whitespace-pre-wrap">
                  {invokeResult}
                </div>
              </div>
            )}

            <div className="flex justify-end gap-2 mt-6">
              <SecondaryButton onClick={() => setIsInvokeModalOpen(false)}>
                Close
              </SecondaryButton>
              <PrimaryButton onClick={executeInvoke}>Run</PrimaryButton>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
