const args = process.argv.slice(2);

if (args.length !== 2 || args[0] !== "-t" || args[1] !== "renders dashboard") {
  console.error(`unexpected unit argv: ${JSON.stringify(args)}`);
  process.exit(1);
}

console.log(`NODE_UNIT_OK:${args.join("|")}`);
