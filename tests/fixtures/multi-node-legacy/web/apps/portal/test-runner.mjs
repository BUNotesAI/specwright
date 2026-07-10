import path from "node:path";

const args = process.argv.slice(2);
if (path.basename(process.cwd()) !== "portal") {
  console.error(`unexpected portal cwd: ${process.cwd()}`);
  process.exit(42);
}
if (args.length !== 2 || args[0] !== "-t" || args[1] !== "portal checkout") {
  console.error(`unexpected portal argv: ${JSON.stringify(args)}`);
  process.exit(43);
}

console.log(`PORTAL_ROUTE_OK:${process.cwd()}:${args.join("|")}`);
console.log(" Test Files  1 passed (1)");
console.log("      Tests  1 passed (1)");
