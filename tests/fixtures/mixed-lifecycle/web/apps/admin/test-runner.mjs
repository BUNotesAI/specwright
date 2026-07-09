const args = process.argv.slice(2);

if (args.length !== 2 || args[0] !== "-t" || args[1] !== "admin smoke") {
  console.error(`unexpected admin argv: ${JSON.stringify(args)}`);
  process.exit(1);
}

console.log(`MIXED_ADMIN_OK:${args.join("|")}`);
console.log(" Test Files  1 passed (1)");
console.log("      Tests  1 passed (1)");
