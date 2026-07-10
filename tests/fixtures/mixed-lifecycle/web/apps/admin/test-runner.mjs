const args = process.argv.slice(2);

if (args.length !== 2 || args[0] !== "-t") {
  console.error(`unexpected admin argv: ${JSON.stringify(args)}`);
  process.exit(1);
}

if (args[1] === "admin smoke") {
  console.log(`MIXED_ADMIN_OK:${args.join("|")}`);
  console.log(" Test Files  1 passed (1)");
  console.log("      Tests  1 passed (1)");
} else if (args[1] === "missing admin smoke") {
  console.log(`MIXED_ADMIN_FILTER_MISS:${args.join("|")}`);
  console.log(" Test Files  1 skipped (1)");
  console.log("      Tests  2 skipped (2)");
} else {
  console.error(`unexpected admin filter: ${JSON.stringify(args[1])}`);
  process.exit(1);
}
